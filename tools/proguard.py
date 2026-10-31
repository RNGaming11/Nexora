#!/usr/bin/env python3
"""Parse Mojang's official ProGuard mappings and translate JVM symbols."""
from dataclasses import dataclass, field
import re

_PRIMITIVES = {"void":"V","boolean":"Z","byte":"B","char":"C","short":"S","int":"I","long":"J","float":"F","double":"D"}

def _descriptor_type(name, source_to_obfuscated):
    dimensions = 0
    while name.endswith("[]"):
        dimensions += 1
        name = name[:-2]
    if name in _PRIMITIVES:
        base = _PRIMITIVES[name]
    else:
        obfuscated = source_to_obfuscated.get(name, name)
        base = "L" + obfuscated.replace(".", "/") + ";"
    return "[" * dimensions + base

def _method_descriptor(return_type, arguments, source_to_obfuscated):
    args = "".join(_descriptor_type(item.strip(), source_to_obfuscated) for item in arguments.split(",") if item.strip())
    return "(" + args + ")" + _descriptor_type(return_type.strip(), source_to_obfuscated)

def remap_descriptor(descriptor, class_map):
    return re.sub(r"L([^;]+);", lambda match: "L" + class_map.get(match.group(1), match.group(1)) + ";", descriptor)

@dataclass
class ProguardMappings:
    obfuscated_to_source: dict = field(default_factory=dict)
    methods: dict = field(default_factory=dict)
    fields: dict = field(default_factory=dict)

    @classmethod
    def read(cls, path):
        result = cls()
        current_source = current_obfuscated = None
        pending = []
        with open(path, encoding="utf-8-sig") as stream:
            for raw in stream:
                line = raw.rstrip("\r\n")
                if not line or line.lstrip().startswith("#"):
                    continue
                if not line[0].isspace():
                    if " -> " not in line or not line.endswith(":"):
                        raise ValueError(f"Invalid ProGuard class mapping: {line[:160]}")
                    source, obfuscated = line[:-1].split(" -> ", 1)
                    current_source, current_obfuscated = source.strip(), obfuscated.strip()
                    if not current_source or not current_obfuscated:
                        raise ValueError(f"Empty class name in mapping: {line!r}")
                    obf = current_obfuscated.replace(".", "/")
                    src = current_source.replace(".", "/")
                    previous = result.obfuscated_to_source.get(obf)
                    if previous is not None and previous != src:
                        raise ValueError(f"Duplicate obfuscated class mapping: {obfuscated}")
                    result.obfuscated_to_source[obf] = src
                    continue
                if current_source is None or " -> " not in line:
                    raise ValueError(f"Invalid ProGuard member mapping: {line[:160]}")
                left, obfuscated_name = (part.strip() for part in line.split(" -> ", 1))
                left = re.sub(r"^\d+:\d+:", "", left)
                left = re.sub(r":\d+:\d+$", "", left)
                pending.append((current_source, current_obfuscated, left, obfuscated_name))
        if not result.obfuscated_to_source:
            raise ValueError(f"No class mappings found in {path}")
        source_to_obfuscated = {source.replace("/", "."): obfuscated.replace("/", ".")
                                for obfuscated, source in result.obfuscated_to_source.items()}
        for source_owner, obfuscated_owner, left, obfuscated_name in pending:
            owner = obfuscated_owner.replace(".", "/")
            method = re.fullmatch(r"(.+?)\s+([^\s(]+)\((.*)\)", left)
            if method:
                return_type, source_name, args = method.groups()
                descriptor = _method_descriptor(return_type, args, source_to_obfuscated).replace(".", "/")
                result.methods[(owner, obfuscated_name, descriptor)] = source_name
                continue
            field = re.fullmatch(r"(.+?)\s+([^\s]+)", left)
            if field:
                field_type, source_name = field.groups()
                descriptor = _descriptor_type(field_type, source_to_obfuscated).replace(".", "/")
                result.fields[(owner, obfuscated_name, descriptor)] = source_name
                continue
            raise ValueError(f"Unrecognized member mapping: {left[:160]}")
        return result

    def class_name(self, name):
        return self.obfuscated_to_source.get(name, name)

    def descriptor(self, descriptor):
        return remap_descriptor(descriptor, self.obfuscated_to_source)

    def method_name(self, owner, name, descriptor):
        return self.methods.get((owner, name, descriptor), name)

    def field_name(self, owner, name, descriptor):
        return self.fields.get((owner, name, descriptor), name)
