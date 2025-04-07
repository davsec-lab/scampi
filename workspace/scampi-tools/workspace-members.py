import tomllib
import sys

args = sys.argv

workspace_name = args[1]
workspace_path = args[2]

manifest_path = f"{workspace_path}/Cargo.toml"

with open(manifest_path, "rb") as f:
    manifest = tomllib.load(f)

members = []

for member in manifest["workspace"]["members"]:
    i = member.rfind('/')

    if i == -1:
        members.append(member)
    else:
        members.append(member[i+1:])

with open(f"{workspace_name}.members.txt", "w") as f:
    f.write("\n".join(members))