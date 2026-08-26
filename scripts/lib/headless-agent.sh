#!/usr/bin/env bash
# The one place a headless CLI is invoked. Sourced, never executed.
#
# Sourced by scripts/request-review.sh and scripts/adversarial-review.sh. It exists because those
# two used to each spell the invocation out, and they DRIFTED: request-review.sh documented that
# kimi-code had dropped --quiet and --work-dir, while adversarial-review.sh three files away was
# still passing both, so the headless driver died on any machine where `kimi` was kimi-code and no
# kimi-cli existed. Two copies of one rule, and nothing holding them together.
#
# Invocation, each verified by running it (kimi-code 0.38.0 and kimi-cli, Aug 2026):
#   claude:    prompt piped on stdin to `claude -p`, response on stdout.
#   kimi-code: prompt passed to `kimi -p` with --output-format text. -p is already
#              non-interactive and cannot be combined with --auto.
#   kimi-cli:  the LEGACY CLI, a different dialect — `--quiet` (its alias for
#              `--print --output-format text --final-message-only`) with the prompt on STDIN.
#              It rejects the kimi-code spelling outright: "Output format is only supported for
#              print UI". Two dialects really do exist, so both live here, in the one place that
#              knows about invoking these CLIs at all. Which one is in hand is decided by asking
#              the binary, not by its name — kimi-cli can be installed AS `kimi`.
# KIMI's tool use is scoped by running it with its cwd set to the scope directory, which works for
# either dialect and outlived kimi-code's removal of --work-dir. CLAUDE is NOT scoped: it runs in
# the caller's directory, as it did before these two scripts shared this file. That is the
# long-standing behaviour rather than a decision taken here, and moving it is a change to how
# claude-authored runs see the project — worth doing deliberately, not as a side effect of a
# refactor. Until then <scope-dir> is simply unused on the claude path, and no comment in this
# tree should claim otherwise.

# A wedged CLI used to hang the driver — and an interactive Bash call — for as long as anyone
# left it. Generous, because a real review of a real diff is not quick.
HEADLESS_AGENT_TIMEOUT="${HEADLESS_AGENT_TIMEOUT:-1800}"

# kimi takes its prompt as an argv ARGUMENT, and Linux caps a single argument at MAX_ARG_STRLEN
# (131072 bytes). The prompts that reach here are a framing document plus a whole diff, and the
# gate that routes work here fires precisely on big diffs — so the review could not be run on
# exactly the changes it exists for, dying with "Argument list too long". Measured at 57,477
# bytes (44% of the cap) on a routine 2,669-line diff. Past the limit the prompt is handed over
# as a file instead. Headroom below the cap for the rest of the argv.
HEADLESS_AGENT_ARGV_LIMIT="${HEADLESS_AGENT_ARGV_LIMIT:-100000}"

# Prefer kimi-code, the current CLI; fall back to a legacy kimi-cli install. Override with
# KIMI_BIN. Whichever is found, its dialect is probed rather than assumed.
headless_kimi_binary() {
    printf '%s' "${KIMI_BIN:-$(command -v kimi || command -v kimi-cli || true)}"
}

# "legacy" (kimi-cli, prompt on stdin) or "current" (kimi-code, prompt in argv). Asked of the
# binary because the `kimi` name can be either one, and guessing wrong is a hard failure on every
# review. A binary that cannot even be asked is an error, not a vote for the default: reporting
# "current" for an install too broken to print its own help would send every review down a path
# it was never going to survive, and blame the wrong thing when it died.
headless_kimi_dialect() {
    local help_text
    if ! help_text="$("$1" --help 2>&1)" || [ -z "$help_text" ]; then
        echo "error: '$1 --help' failed, so its dialect cannot be determined" >&2
        return 1
    fi
    case "$help_text" in
        *--quiet*) printf 'legacy' ;;
        *) printf 'current' ;;
    esac
}

# run_headless_agent <claude|kimi> <prompt-file> <scope-dir>
#
# Runs the prompt in <prompt-file> and writes the response to stdout. For kimi, <scope-dir> is
# where it runs, so any tool call it makes is confined there and it cannot read the surrounding
# code — prompts must carry their own context. For claude, <scope-dir> is unused (see above).
run_headless_agent() {
    local agent="$1" prompt_file="$2" scope_dir="$3"
    local prompt_bytes kimi_binary kimi_dialect handover_name status

    case "$agent" in
        claude)
            command -v claude >/dev/null || { echo "error: 'claude' not on PATH" >&2; return 1; }
            timeout "$HEADLESS_AGENT_TIMEOUT" claude -p < "$prompt_file"
            ;;
        kimi)
            kimi_binary="$(headless_kimi_binary)"
            [ -n "$kimi_binary" ] || { echo "error: kimi not on PATH (set KIMI_BIN)" >&2; return 1; }
            kimi_dialect="$(headless_kimi_dialect "$kimi_binary")" || return 1
            if [ "$kimi_dialect" = "legacy" ]; then
                # stdin: no argv limit to work around, so the prompt goes over whole at any size.
                ( cd "$scope_dir" && timeout "$HEADLESS_AGENT_TIMEOUT" \
                    "$kimi_binary" --quiet < "$prompt_file" )
                return
            fi
            prompt_bytes=$(wc -c < "$prompt_file")
            if [ "$prompt_bytes" -le "$HEADLESS_AGENT_ARGV_LIMIT" ]; then
                ( cd "$scope_dir" && timeout "$HEADLESS_AGENT_TIMEOUT" \
                    "$kimi_binary" --output-format text -p "$(cat "$prompt_file")" )
            else
                # Say so out loud. The agent now decides how much of the file to read, where
                # before it was handed every byte, and a review that quietly skimmed its subject
                # would be worse than one that failed — at least a failure is visible.
                echo "note: prompt is $prompt_bytes bytes, over the ${HEADLESS_AGENT_ARGV_LIMIT}-byte argv limit;" >&2
                echo "      handing it to kimi as a file to read instead of inline." >&2
                # Named for THIS process. A fixed name collided between the two concurrent
                # sessions this tooling is built for — one review reading the other's prompt, or
                # having it deleted mid-read — which is the same race the review-N claim in
                # request-review.sh was fixed to avoid, reopened one file away.
                handover_name=".headless-prompt.$$.txt"
                cp "$prompt_file" "$scope_dir/$handover_name"
                ( cd "$scope_dir" && timeout "$HEADLESS_AGENT_TIMEOUT" \
                    "$kimi_binary" --output-format text -p \
                    "Read the file $handover_name in your current directory. It is $prompt_bytes bytes and it is your complete instructions, including the material under review. Read ALL of it, from the first line to the last, before you answer — a partial read makes the answer worthless. Then do exactly what it says." )
                status=$?
                rm -f "$scope_dir/$handover_name"
                return "$status"
            fi
            ;;
        *)
            echo "error: unknown agent '$agent'" >&2
            return 1
            ;;
    esac
}
