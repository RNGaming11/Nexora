#!/usr/bin/env python3
"""Audit existing typed packet codecs against a generated target packet-ID inventory."""

import argparse
import json
import re
from pathlib import Path

MODULE = re.compile(r"^\s*pub mod ([a-z_]+) \{$")
CONST = re.compile(r"^\s*pub const ([A-Z0-9_]+): i32 = (-?\d+);$")
ENUM = re.compile(
    r"packet_enum!\(\s*(Clientbound|Serverbound)\s*,\s*"
    r"crate::ids::([a-z_]+)::(clientbound|serverbound)\s*\{(.*?)\}\s*\);",
    re.S,
)
VARIANT = re.compile(
    r"\b([A-Za-z][A-Za-z0-9_]*)\s*\(\s*"
    r"((?:(?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*))"
    r"\s*\)\s*=\s*([A-Z0-9_]+)"
)


def read_ids(path):
    result = {}
    state = direction = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        match = MODULE.match(line)
        if match:
            if state is None:
                state = match.group(1)
            elif direction is None:
                direction = match.group(1)
            continue
        if line.strip() == "}":
            if direction is not None:
                direction = None
            elif state is not None:
                state = None
            continue
        match = CONST.match(line)
        if match and state and direction:
            result.setdefault((state, direction), {})[match.group(1)] = {
                "id": int(match.group(2)), "name": match.group(1).lower()
            }
    return result


def read_codecs(root):
    result = {}
    versioned = {
        ("configuration", "clientbound"): {
            "CUSTOM_REPORT_DETAILS": ("CustomReportDetails", "v1_21_1::CustomReportDetails"),
        },
        ("login", "clientbound"): {
            "GAME_PROFILE": ("GameProfilePacket", "v1_21_1::GameProfilePacket"),
        },
        ("play", "clientbound"): {
            "ADD_EXPERIENCE_ORB": ("AddExperienceOrb", "v1_21_1::AddExperienceOrb"),
            "COMMAND_SUGGESTIONS": ("CommandSuggestions", "v1_21_1::CommandSuggestions"),
            "CUSTOM_REPORT_DETAILS": ("CustomReportDetails", "v1_21_1::CustomReportDetails"),
            "DEBUG_SAMPLE": ("DebugSample", "v1_21_1::DebugSample"),
            "HORSE_SCREEN_OPEN": ("HorseScreenOpen", "v1_21_1::HorseScreenOpen"),
            "PLAYER_COMBAT_END": ("PlayerCombatEnd", "v1_21_1::PlayerCombatEnd"),
            "PLAYER_COMBAT_ENTER": ("PlayerCombatEnter", "v1_21_1::Empty"),
            "RECIPE": ("Recipe", "v1_21_1::RecipePacket"),
            "SELECT_ADVANCEMENTS_TAB": ("SelectAdvancementsTab", "v1_21_1::SelectAdvancementsTab"),
            "SET_CARRIED_ITEM": ("SetCarriedItem", "v1_21_1::SetCarriedItem"),
            "UPDATE_RECIPES": ("UpdateRecipes", "v1_21_1::UpdateRecipes (registry-backed)"),
        },
        ("play", "serverbound"): {
            "COMMAND_SUGGESTION": ("CommandSuggestionRequest", "v1_21_1::CommandSuggestionRequest"),
            "DEBUG_SAMPLE_SUBSCRIPTION": ("DebugSampleSubscription", "v1_21_1::DebugSampleSubscription"),
            "PICK_ITEM": ("PickItem", "v1_21_1::PickItem"),
            "RECIPE_BOOK_SEEN_RECIPE": ("RecipeBookSeenRecipe", "v1_21_1::RecipeBookSeenRecipe"),
            "SEEN_ADVANCEMENTS": ("SeenAdvancements", "v1_21_1::SeenAdvancements"),
            "SWING": ("Swing", "v1_21_1::Swing"),
        },
    }
    packet_dir = Path(root) / "crates/horizon-protocol/src/packets"
    for path in packet_dir.glob("*.rs"):
        source = path.read_text(encoding="utf-8")
        for match in ENUM.finditer(source):
            direction = match.group(3)
            state = match.group(2)
            entries = result.setdefault((state, direction), {})
            for variant, packet_type, constant in VARIANT.findall(match.group(4)):
                entries[constant] = {"variant": variant, "type": packet_type, "source": str(path)}
    for key, entries in versioned.items():
        target = result.setdefault(key, {})
        for constant, (variant, packet_type) in entries.items():
            target[constant] = {
                "variant": variant,
                "type": packet_type,
                "source": "crates/horizon-protocol/src/packets/v1_21_1.rs",
                "version": "1.21.1",
                **({"registry_dependent": True} if constant == "UPDATE_RECIPES" else {}),
            }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--target", type=Path, default=Path("ports/1.21.1/packet_ids.rs"))
    parser.add_argument("--out", type=Path, default=Path("ports/1.21.1/packet-codec-coverage.json"))
    args = parser.parse_args()

    target = read_ids(args.target)
    codecs = read_codecs(args.root)
    report = {"target_packet_count": sum(map(len, target.values())), "codec_packet_count": sum(map(len, codecs.values())), "states": {}}
    for key in sorted(target):
        wanted, existing = target[key], codecs.get(key, {})
        missing = sorted(name for name in wanted if name not in existing)
        obsolete = sorted(name for name in existing if name not in wanted)
        report["states"][f"{key[0]}/{key[1]}"] = {
            "target_count": len(wanted),
            "existing_codec_count": len(existing),
            "covered_count": len(wanted) - len(missing),
            "missing_codecs": [{"constant": name, **wanted[name]} for name in missing],
            "codecs_not_in_target": [{"constant": name, **existing[name]} for name in obsolete],
        }
        print(f"{key[0]}/{key[1]}: {len(wanted)-len(missing)}/{len(wanted)} target packets have existing codecs; {len(missing)} need codecs")
        if missing:
            print("  Missing: " + ", ".join(f"{name.lower()} (id {wanted[name]['id']})" for name in missing))
    missing_total = sum(len(v["missing_codecs"]) for v in report["states"].values())
    report["missing_codec_count"] = missing_total
    report["registry_dependent_codecs"] = [
        {"state": f"{state}/{direction}", "constant": name, **details}
        for (state, direction), entries in codecs.items()
        for name, details in entries.items()
        if details.get("registry_dependent")
    ]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Missing target codecs: {missing_total}")
    print(f"Detailed coverage report: {args.out}")


if __name__ == "__main__":
    main()
