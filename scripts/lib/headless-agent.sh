#!/usr/bin/env bash
# The one place a headless CLI is invoked. Sourced, never executed.
#
# Sourced by scripts/request-review.sh and scripts/adversarial-review.sh. It exists because those
# two used to each spell the invocation out, and they DRIFTED: request-review.sh documented that
# kimi-code had dropped --quiet and --work-dir, while adversarial-review.sh three files away was
# still passing both, so the headless driver died on any machine where `kimi` was kimi-code and no
# kimi-cli existed. Two copies of one rule, and nothing holding them together.
#
# Invocation, each verified by running it (kimi-code 0.36, kimi-cli Aug 2026, agy 1.1.24 on
# 2026-09-02):
#   claude:    prompt piped on stdin to `claude -p`, response on stdout.
#   kimi-code: prompt passed to `kimi -p` with --output-format text. -p is already
#              non-interactive and cannot be combined with --auto or --plan.
#   kimi-cli:  the LEGACY CLI, a different dialect — `--quiet` (its alias for
#              `--print --output-format text --final-message-only`) with the prompt on STDIN.
#              It rejects the kimi-code spelling outright: "Output format is only supported for
#              print UI". Two dialects really do exist, so both live here, in the one place that
#              knows about invoking these CLIs at all. Which one is in hand is decided by asking
#              the binary, not by its name — kimi-cli can be installed AS `kimi`.
#   gemini:    the Antigravity CLI, `agy`, pinned to a Gemini model. The name is the vendor, not
#              the binary, because the rule it serves is "a different vendor's model reviews": agy
#              also offers Claude models, and this arm must never pick one. The prompt goes over
#              agy's NDJSON stdin (--input-format stream-json), which carried a 186 KB prompt
#              intact, so there is no argv limit to work around; the review is the `result`
#              event's response.
#
# WHAT "READ-ONLY" MEANS, PER REVIEWER — each line below was proven by forcing tool calls by
# name and then looking at the directory, not by asking the model what it had:
#
#   kimi   tool set ENFORCED by prompts/kimi-headless-agent.md (--agent-file): its frontmatter
#          `tools:` allowlist is what the model's function schema declares, and kimi-code's docs
#          call it an enforcement point applied before execution. Forced to emit Bash and the write
#          tools, it reported them undeclared and emitted nothing. Paths are ADVISORY: nothing
#          restricts what Read may open. The repo is added with --add-dir; cwd is the subject dir.
#          The agent file is role-NEUTRAL on purpose: adversarial-review.sh runs kimi as the AUTHOR
#          too, and an author framed as "the reviewer" defends nothing (gemini's code review, F1).
#          Neither role may change files, so both get the same read-only tool set.
#   agy    tool set AND paths ENFORCED by a PreToolUse hook this file GENERATES into the subject
#          directory on every run (write_agy_gate): four read tools allowed, and only on a path
#          that resolves under <repo-dir>; everything else hard-denied with a reason the model
#          recovers from. Agy loads .agents/hooks.json only from a directory passed as --add-dir —
#          never from a bare cwd and never from the repo root — so the subject dir is passed as its
#          own --add-dir. Rejected on the way here, each by a probe: agy's plan mode (does not
#          enforce), its custom agents' `tools:` list (a hint — run_command executed when forced),
#          --dangerously-skip-permissions (turns that hint into full privilege), and bare headless
#          mode (writes inside the workspace are auto-allowed).
#   claude NOT scoped: `claude -p` runs in the caller's directory with read access, as it did
#          before these scripts shared this file. Moving it is a change to how claude-authored runs
#          see the project — worth doing deliberately, not as a side effect. <repo-dir> is unused
#          on that path, and no comment in this tree should claim otherwise.
#
# TWO PER-MACHINE FACTS ABOUT agy 1.1.24 that this file cannot fix (the adversarial-review skill
# carries the setup):
#   - grep_search asks for the read_file permission on any path under the home directory, and a
#     headless run answers any permission question by CANCELLING — with status SUCCESS and an empty
#     response. The fix is one allow rule in ~/.gemini/antigravity-cli/settings.json:
#         {"permissions": {"allow": ["read_file(/home/<user>/projects)"]}}
#     naming the directory that holds the clones. It doubles as an outer path fence.
#   - its embedded ripgrep is linked against a Google-internal loader (/usr/grte/v5/...) and cannot
#     execute on a stock Linux; agy then silently falls back to a shell grep. With the allow rule
#     above that fallback WORKS, but it does not honour .gitignore, so a search over a repo with
#     node_modules/ or target/ returns their contents too and the reviewer pays for reading them.
#     headless_agy_ripgrep_warning says so when it sees it. Copying a working rg over the extracted
#     file does NOT help — agy re-extracts on the next run — and an rg on PATH is ignored; the only
#     real fix is the loader itself (a sudo symlink, see the skill) or an agy release that ships a
#     portable binary.

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

# Pinned, not left to agy's default: the point of a second reviewer is a NAMED model from a
# different vendor, and agy's default is whatever Google promotes that week. Must be a Gemini
# slug (see the header); `agy models` lists them.
HEADLESS_AGY_MODEL="${HEADLESS_AGY_MODEL:-gemini-3.8-flash-high}"

# The kimi agent file, resolved from this file's own location so each repo's copy finds its own.
HEADLESS_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HEADLESS_KIMI_AGENT="${HEADLESS_KIMI_AGENT:-$HEADLESS_LIB_DIR/../../prompts/kimi-headless-agent.md}"

# Prefer kimi-code, the current CLI; fall back to a legacy kimi-cli install. Override with
# KIMI_BIN. Whichever is found, its dialect is probed rather than assumed.
headless_kimi_binary() {
    printf '%s' "${KIMI_BIN:-$(command -v kimi || command -v kimi-cli || true)}"
}

# The Antigravity CLI. Override with AGY_BIN.
headless_agy_binary() {
    printf '%s' "${AGY_BIN:-$(command -v agy || true)}"
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

# Say so before the run: an agy whose extracted ripgrep cannot execute falls back to a grep that
# ignores .gitignore, and the reviewer pays quota for node_modules/ and the like (see the header).
headless_agy_ripgrep_warning() {
    local extracted
    for extracted in "$HOME"/.cache/antigravity/bin/rg_embedded-*; do
        [ -f "$extracted" ] || continue
        if ! "$extracted" --version >/dev/null 2>&1; then
            echo "warning: agy's extracted ripgrep at $extracted cannot execute on this machine;" >&2
            echo "         grep_search falls back to a grep that ignores .gitignore, so searches cost more quota." >&2
            echo "         The adversarial-review skill's per-machine setup names the fix." >&2
        fi
    done
}

# write_agy_gate <scope-dir> <repo-dir>
#
# The read-only gate for agy, generated fresh on every run so its presence is never a proxy for
# its content. hooks.json registers it for every tool (matcher "*"); the gate allows the four read
# tools when their path argument resolves under <repo-dir> and denies everything else. agy runs the
# gate with cwd = .agents/ and the tool call as JSON on stdin; the argument names are the ones agy
# actually sends (AbsolutePath, SearchDirectory, SearchPath, DirectoryPath).
#
# The root is canonicalised the same way the gate canonicalises each path — realpath — or a home
# directory reached through a symlink (/home -> /var/home on ostree systems) would deny every read
# (gemini's code review, F2). A single quote in the path is escaped for the single-quoted
# assignment the gate carries it in (F6).
write_agy_gate() {
    local scope_dir="$1" repo_dir="$2" allowed_root
    allowed_root="$(realpath -m -- "$repo_dir")"
    allowed_root="${allowed_root//\'/\'\\\'\'}"
    mkdir -p "$scope_dir/.agents"
    cat > "$scope_dir/.agents/hooks.json" <<'HOOKS'
{
  "adversarial-reviewer-gate": {
    "PreToolUse": [
      { "matcher": "*", "hooks": [ { "type": "command", "command": "./reviewer-gate.sh", "timeout": 10 } ] }
    ]
  }
}
HOOKS
    cat > "$scope_dir/.agents/reviewer-gate.sh" <<GATE
#!/usr/bin/env bash
# Generated by scripts/lib/headless-agent.sh (write_agy_gate) for one headless review run.
# Read-only gate for the agy reviewer: four read tools, only under the allowed root; all else denied.
allowed_root='$allowed_root'
input=\$(cat)
name=\$(printf '%s' "\$input" | jq -r '.toolCall.name // ""')
path=\$(printf '%s' "\$input" | jq -r '.toolCall.args | (.AbsolutePath // .SearchDirectory // .SearchPath // .DirectoryPath // "")')
# Decisions are built by jq, not printf: a path with a double quote in it must not turn a soft
# denial into malformed JSON and a hook error.
deny() { jq -cn --arg reason "\$1" '{decision:"deny", reason:\$reason}'; }
case "\$name" in
    view_file|find_by_name|grep_search|list_dir)
        if [ -z "\$path" ]; then
            deny "read-only reviewer: \$name without a path argument"
            exit 0
        fi
        resolved=\$(realpath -m -- "\$path")
        case "\$resolved" in
            "\$allowed_root"|"\$allowed_root"/*) printf '{"decision":"allow"}\n' ;;
            *) deny "read-only reviewer: \$name may only read under \$allowed_root, not \$resolved" ;;
        esac ;;
    *) deny "read-only reviewer: \$name is not permitted" ;;
esac
GATE
    chmod +x "$scope_dir/.agents/reviewer-gate.sh"
}

# The read scope, said out loud before every run. From the working-set root that scope holds every
# nested clone, private ones included, and a diff that touches only scripts/ still exposes them —
# so the note names each nested git repo it finds, and the exposure is a visible decision rather
# than an innocuous line (kimi's code review, F1).
headless_read_scope_note() {
    local agent="$1" repo_dir="$2" nested
    nested="$(find "$repo_dir" -mindepth 2 -maxdepth 3 -name .git -prune 2>/dev/null \
        | sed "s|^$repo_dir/||; s|/\.git$||" | sort | tr '\n' ' ')"
    echo "note: $agent may read $repo_dir, read-only" >&2
    [ -z "$nested" ] || echo "      including the nested clones: $nested" >&2
}

# run_headless_agent <claude|kimi|gemini> <prompt-file> <scope-dir> <repo-dir>
#
# Runs the prompt in <prompt-file> and writes the response to stdout. <scope-dir> is where the
# agent runs — the subject directory, where its own working files land. <repo-dir> is the tree it
# may read, added as a read-only workspace (see the header for what each CLI enforces). For claude
# both are unused. The agent may be the reviewer or, in adversarial-review.sh, the author: the
# read-only arrangement is the same for both, since neither may change files.
run_headless_agent() {
    local agent="$1" prompt_file="$2" scope_dir="$3" repo_dir="$4"
    local prompt_bytes kimi_binary kimi_dialect handover_name status
    local agy_binary ndjson agy_status agy_response
    # Every arm that cd's into <scope-dir> must still find the prompt where the caller left it.
    prompt_file="$(realpath -m -- "$prompt_file")"

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
                # The legacy CLI has neither --agent-file nor --add-dir: it stays confined to the
                # subject directory by cwd alone, which is the old, advisory arrangement.
                ( cd "$scope_dir" && timeout "$HEADLESS_AGENT_TIMEOUT" \
                    "$kimi_binary" --quiet < "$prompt_file" )
                return
            fi
            [ -f "$HEADLESS_KIMI_AGENT" ] || { echo "error: kimi agent file not found: $HEADLESS_KIMI_AGENT" >&2; return 1; }
            headless_read_scope_note "kimi (read tools only; paths by instruction)" "$repo_dir"
            prompt_bytes=$(wc -c < "$prompt_file")
            if [ "$prompt_bytes" -le "$HEADLESS_AGENT_ARGV_LIMIT" ]; then
                ( cd "$scope_dir" && timeout "$HEADLESS_AGENT_TIMEOUT" \
                    "$kimi_binary" --agent-file "$HEADLESS_KIMI_AGENT" --add-dir "$repo_dir" \
                    --output-format text -p "$(cat "$prompt_file")" )
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
                    "$kimi_binary" --agent-file "$HEADLESS_KIMI_AGENT" --add-dir "$repo_dir" \
                    --output-format text -p \
                    "Read the file $handover_name in your current directory. It is $prompt_bytes bytes and it is your complete instructions, including the material under review. Read ALL of it, from the first line to the last, before you answer — a partial read makes the answer worthless. Then do exactly what it says." )
                status=$?
                rm -f "$scope_dir/$handover_name"
                return "$status"
            fi
            ;;
        gemini)
            agy_binary="$(headless_agy_binary)"
            [ -n "$agy_binary" ] || { echo "error: agy not on PATH (set AGY_BIN)" >&2; return 1; }
            command -v jq >/dev/null || { echo "error: jq not on PATH; the gemini arm needs it" >&2; return 1; }
            # This arm does arithmetic on the timeout (the margin below) where the others hand it
            # to `timeout` verbatim, so a value like 45m that has always worked there must be
            # refused here with the reason, not a bash arithmetic error.
            [[ "$HEADLESS_AGENT_TIMEOUT" =~ ^[0-9]+$ ]] || { echo "error: HEADLESS_AGENT_TIMEOUT must be a whole number of seconds for the gemini arm (got '$HEADLESS_AGENT_TIMEOUT')" >&2; return 1; }
            # The generated gate resolves every path with GNU realpath; without it every read would
            # be denied and the failure would look like a permission problem.
            realpath -m -- / >/dev/null 2>&1 || { echo "error: 'realpath -m' not available; the gemini arm's gate needs GNU realpath" >&2; return 1; }
            headless_agy_ripgrep_warning
            write_agy_gate "$scope_dir" "$repo_dir"
            headless_read_scope_note "gemini (agy, $HEADLESS_AGY_MODEL; gate written to $scope_dir/.agents/)" "$repo_dir"
            # Named for THIS process, for the same reason as kimi's handover file above.
            ndjson="$scope_dir/.agy-output.$$.ndjson"
            # The outer watchdog gets a margin over agy's own --print-timeout, so agy times out
            # first and still emits its result event instead of being killed mid-stream.
            (
                set -o pipefail
                cd "$scope_dir" && jq -Rs '{event:"user", message:{content:.}}' "$prompt_file" \
                    | timeout "$((HEADLESS_AGENT_TIMEOUT + 30))" "$agy_binary" \
                        --add-dir "$scope_dir" --model "$HEADLESS_AGY_MODEL" \
                        --print-timeout "${HEADLESS_AGENT_TIMEOUT}s" \
                        --input-format stream-json --output-format stream-json > "$ndjson"
            ) || { echo "error: agy exited non-zero; its output is kept at $ndjson" >&2; return 1; }
            # A headless run that was refused a permission ends with status SUCCESS and an EMPTY
            # response, so success is "SUCCESS and something to say", not the status alone. Both
            # fields come from the LAST result event, the same one, so a run that emitted more than
            # one can neither concatenate responses nor pair one event's status with another's text.
            agy_status="$(jq -rs '[.[] | select(.event == "result")] | last | .result.status // empty' "$ndjson")"
            agy_response="$(jq -rs '[.[] | select(.event == "result")] | last | .result.response // empty' "$ndjson")"
            if [ "$agy_status" != "SUCCESS" ] || [ -z "$agy_response" ]; then
                echo "error: agy produced no review (status '${agy_status:-none}'); its output is kept at $ndjson" >&2
                jq -rs '[.[] | select(.event == "result")] | last | .result.error // empty' "$ndjson" >&2
                echo "      an empty SUCCESS usually means a permission question headless mode could not ask (see the" >&2
                echo "      per-machine setup in the adversarial-review skill), or that the gate in $scope_dir/.agents/" >&2
                echo "      denied every read — the step_update events in the kept output say which." >&2
                return 1
            fi
            printf '%s\n' "$agy_response"
            rm -f "$ndjson"
            ;;
        *)
            echo "error: unknown agent '$agent'" >&2
            return 1
            ;;
    esac
}
