//! Map the exported score through the same validated API as the Conductor panel.
use editor::song::{Gesture, SongDocument, SongEdit};
use music_midi::{
    Binding, Instrument, InstrumentSource, Part, PartMode, Provenance, Quantize, TrackSelector,
};
use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let path = assets.join("ai/music/ai_food_pyramid.mid");
    if !path.with_extension("mid.ron").is_file() {
        return Err("existing sidecar required; restore it before installing the new score".into());
    }
    let mut document = SongDocument::open(&path, &assets)?.document;
    for action in ["brick", "armor", "paddle", "loss", "clear", "menu"] {
        if !document.file().bindings.iter().any(|binding| binding.action == action) {
            return Err(format!("existing {action} binding required; restore the complete sidecar").into());
        }
    }
    let bank = "soundfonts/ai_food_pyramid.sf2";
    let mut edits = vec![
        SongEdit::SetSoundFont { path: bank.into() },
        SongEdit::SetProvenance {
            provenance: Provenance {
                author: "Original AI composition and MIDI Evil synthesis for Be Insiculous, 2026"
                    .into(),
                license: "CC0-1.0".into(),
            },
        },
    ];
    for instrument in &document.file().instruments {
        let mut replacement = instrument.clone();
        if let InstrumentSource::SoundFont { path, .. } = &mut replacement.source {
            *path = bank.into();
        }
        edits.push(SongEdit::SetInstrument {
            name: instrument.name.clone(),
            instrument: replacement,
        });
    }
    let file = document.file();
    let part_exists = file.parts.iter().any(|part| part.name == "movement");
    let binding = file.bindings.iter().find(|binding| binding.action == "move");
    let complete = part_exists && binding.is_some_and(|binding| {
        binding.part == "movement" && file.instruments.iter().any(|instrument| instrument.name == binding.instrument)
    });
    if !complete && (part_exists || binding.is_some() || file.instruments.iter().any(|instrument| instrument.name == "movement")) {
        return Err("partial movement map; repair it in the Conductor panel before installation".into());
    }
    if !complete {
        edits.extend([
            SongEdit::AddPart {
                part: Part {
                    name: "movement".into(),
                    track: TrackSelector::Name("movement".into()),
                    mode: PartMode::Loop,
                    quantize: Quantize::Off,
                },
            },
            SongEdit::AddInstrument {
                instrument: Instrument {
                    name: "movement".into(),
                    provenance: None,
                    source: InstrumentSource::SoundFont {
                        path: bank.into(),
                        bank: 0,
                        preset: 59,
                    },
                },
            },
            SongEdit::SetBinding {
                binding: Binding {
                    action: "move".into(),
                    part: "movement".into(),
                    instrument: "movement".into(),
                },
            },
        ]);
    }
    document.apply(SongEdit::Batch(edits), Gesture::Discrete)?;
    document.save(&path, &assets)?;
    println!("saved {} through SongDocument", path.display());
    Ok(())
}
