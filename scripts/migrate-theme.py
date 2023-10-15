#!/usr/bin/env python3

import re
import sys


def migrate_color(line: str) -> str:
    def as_hex(number_str: str) -> int:
        number = int(number_str)
        return f"{number:02x}"

    matches = re.match(r'.*"rgb_\(([\d]+),([\d]+),([\d]+)\)"', line)
    if not matches:
        return line
    hex_color = "".join(map(as_hex, matches.group(1, 2, 3)))
    original_color = f"rgb_({matches[1]},{matches[2]},{matches[3]})"
    return line.replace(original_color, hex_color)


def migrate(line: str) -> str:
    migrators = [migrate_color]
    for migrator in migrators:
        line = migrator(line)
    return line


for line in sys.stdin:
    new_line = migrate(line)
    sys.stdout.write(new_line)
