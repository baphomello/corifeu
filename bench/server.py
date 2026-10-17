"""Print the decoding configuration a round actually ran against.

    bench/server.py 192.168.32.1:9931

A seed without the temperature beside it says nothing. Round seven ran eight
passes at one seed base, four greedy and four sampled, and the stored headers
were identical apart from their timestamps — so the record could not tell which
pass was which. The temperature comes from the request when the harness sets
one and from the server when it does not, and whether speculative decoding is
on is reported on `/slots` and nowhere else.

Prints one line, or says what it could not reach. Never fails the round.
"""

import json
import sys
import urllib.request


def get(host, path):
    with urllib.request.urlopen(f"http://{host}{path}", timeout=5) as response:
        return json.load(response)


def sampling(host):
    props = get(host, "/props")
    general = props["default_generation_settings"]
    p = general["params"]
    return (f"n_ctx={general['n_ctx']} temp={p['temperature']} top_k={p['top_k']} "
            f"top_p={p['top_p']:.2f} min_p={p['min_p']} build={props['build_info']}")


def speculative(host):
    slots = get(host, "/slots")
    slot = slots[0] if isinstance(slots, list) else slots
    if not slot.get("speculative"):
        return "off"
    return slot.get("params", {}).get("speculative.types", "on, type unknown")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    host = sys.argv[1]
    for name, fn in (("server", sampling), ("spec", speculative)):
        try:
            print(f"{name}\t{fn(host)}")
        except Exception as error:
            print(f"{name}\tunreadable: {type(error).__name__}")
