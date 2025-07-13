import re

data = """
src/cache.rs: 0/53
src/colors.rs: 142/152
src/config.rs: 0/7
src/display/formatter.rs: 0/139
src/lyrics/api.rs: 0/21
src/lyrics/cache.rs: 0/31
src/lyrics/mod.rs: 0/6
src/lyrics/parser.rs: 31/31
src/main.rs: 0/160
src/models/mod.rs: 32/32
src/player.rs: 25/82
src/progress.rs: 10/10
src/providers/tidal/api.rs: 0/121
src/providers/tidal/auth.rs: 2/23
src/providers/tidal/mod.rs: 0/4
src/ui/mod.rs: 0/336
"""

print("Code Coverage Summary")
print("=" * 60)
print()
print(f"{'Module':<30} {'Coverage':>10} {'Lines':>15}")
print("-" * 60)

total_covered = 0
total_lines = 0

for line in data.strip().split('\n'):
    if line:
        module, coverage = line.split(': ')
        covered, total = map(int, coverage.split('/'))
        percentage = (covered / total * 100) if total > 0 else 0
        total_covered += covered
        total_lines += total
        print(f"{module:<30} {percentage:>9.1f}% {covered:>6}/{total:<6}")

print("-" * 60)
overall = (total_covered / total_lines * 100) if total_lines > 0 else 0
print(f"{'TOTAL':<30} {overall:>9.1f}% {total_covered:>6}/{total_lines:<6}")
print()
print("Modules with 100% coverage:")
for line in data.strip().split('\n'):
    if line:
        module, coverage = line.split(': ')
        covered, total = map(int, coverage.split('/'))
        if covered == total and total > 0:
            print(f"  ✓ {module}")
