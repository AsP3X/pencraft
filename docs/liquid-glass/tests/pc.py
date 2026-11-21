#!/usr/bin/env python3
"""Call one of Quill's agent tools from a shell, the way an MCP client would.

    python pc.py TOOL ['{"json": "arguments"}' | @arguments.json]  [--out picture.png]

The tool's text answers are printed; a picture it returns is saved to --out (default: pc-picture.png in the
current folder) and its path printed. `python pc.py tools` lists the tools with their descriptions.

Quill must be running. Its address and secret are read from $PENCRAFT_HOME/.pencraft/agent.json
(PENCRAFT_HOME defaults to the home folder), which Quill writes at start. See README.md beside this file.
"""
import json
import os
import sys
import base64
import urllib.request


def call(method, params):
    home = os.environ.get("PENCRAFT_HOME") or os.path.expanduser("~")
    with open(os.path.join(home, ".pencraft", "agent.json"), encoding="utf-8") as file:
        info = json.load(file)
    url = info.get("url") or "http://127.0.0.1:%s/mcp" % info["port"]
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    request = urllib.request.Request(url, body, {"Content-Type": "application/json", "Authorization": "Bearer " + info["token"]})
    try:
        with urllib.request.urlopen(request, timeout=180) as answer:
            reply = json.load(answer)
    except OSError as error:
        sys.exit("error: Quill can't be reached (%s). Is it running?" % error)
    if "error" in reply:
        sys.exit("error: " + reply["error"]["message"])
    return reply["result"]


def main():
    args = sys.argv[1:]
    out = "pc-picture.png"
    if "--out" in args:
        at = args.index("--out")
        out = args[at + 1]
        del args[at:at + 2]
    if not args:
        sys.exit(__doc__)
    if args[0] == "tools":
        for tool in call("tools/list", {})["tools"]:
            print("%s\n    %s\n    arguments: %s\n" % (tool["name"], tool["description"], json.dumps(tool["inputSchema"].get("properties", {}))))
        return
    given = args[1] if len(args) > 1 else "{}"
    if given.startswith("@"):
        with open(given[1:], encoding="utf-8") as file:
            given = file.read()
    result = call("tools/call", {"name": args[0], "arguments": json.loads(given)})
    for part in result.get("content", []):
        if part.get("type") == "image":
            with open(out, "wb") as file:
                file.write(base64.b64decode(part["data"]))
            print("[picture saved to %s]" % os.path.abspath(out))
        else:
            print(part.get("text", ""))
    if result.get("isError"):
        sys.exit(1)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
