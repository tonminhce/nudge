import re
import sys


def read(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def split_statements(text):
    blocks = []
    current = []
    for line in text.splitlines():
        if not line.strip():
            continue
        if line.startswith(("--", "\\restrict", "SET ", "SELECT pg_catalog", "\\warn")):
            continue
        if re.match(r"^\s*ALTER\s+(TABLE|SCHEMA|SEQUENCE|TYPE|VIEW)\s+.*\bOWNER\s+TO\b", line):
            continue
        current.append(line)
        if line.rstrip().endswith(";"):
            blocks.append("\n".join(current))
            current = []
    return blocks


def normalize(block, schema, sort_columns_mode):
    out = block.replace(f'"{schema}"', "ws").replace(f"{schema}.", "ws.").replace(schema, "ws")
    out = re.sub(r"\bIDX_ACCESS_GROUP_IDS_[0-9a-f]{24,32}\b", "IDX_ACCESS_GROUP_IDS_<obj>", out)
    out = re.sub(r"\bIDX_PRIVATE_RECORD_[0-9a-f]{24,32}\b", "IDX_PRIVATE_RECORD_<obj>", out)
    if sort_columns_mode:
        out = re.sub(
            r"(CREATE TABLE [^;(]+\()(\s*((?:[^;]|\n)*))(^\);)",
            lambda m: m.group(1) + sort_columns(m.group(2)) + "\n);",
            out,
            flags=re.S | re.M,
        )
    return out


def sort_columns(body):
    lines = [l.strip().rstrip(",") for l in body.splitlines() if l.strip()]
    lines.sort()
    return "\n" + "\n".join(lines) + "\n"


def statements(path, schema, sort_columns_mode):
    facts = set()
    for block in split_statements(read(path)):
        facts.add(normalize(block, schema, sort_columns_mode))
    return facts


def main():
    if len(sys.argv) != 6:
        print("usage: dump-normalize.py <baseline.sql> <baseline-schema> <dump.sql> <dump-schema> <sort:0|1>")
        return 2
    sort_mode = sys.argv[5] == "1"
    baseline = statements(sys.argv[1], sys.argv[2], sort_mode)
    dumped = statements(sys.argv[3], sys.argv[4], sort_mode)
    only_base = baseline - dumped
    only_dump = dumped - baseline
    print(f"baseline statements: {len(baseline)}")
    print(f"dumped statements:   {len(dumped)}")
    print(f"only in baseline:    {len(only_base)}")
    print(f"only in dump:        {len(only_dump)}")
    for label, group in (("BASELINE", sorted(only_base)), ("DUMP", sorted(only_dump))):
        for item in group[:25]:
            head = item.splitlines()[0]
            extra = f" (+{len(item.splitlines()) - 1} lines)" if "\n" in item else ""
            print(f"  [{label}] {head[:160]}{extra}")
    return 0 if not only_base and not only_dump else 1


if __name__ == "__main__":
    sys.exit(main())
