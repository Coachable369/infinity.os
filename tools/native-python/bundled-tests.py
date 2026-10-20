"""Execute real bundled package operations inside the native interpreter."""
import sys
import io
import json
import importlib.metadata as metadata
from _infinity_inventory import INVENTORY

for name, version in INVENTORY["bundled_distributions"].items():
    assert metadata.version(name) == version, name
assert len(INVENTORY["bundled_distributions"]) >= 22

from packaging.requirements import Requirement
from packaging.version import Version
from packaging.markers import default_environment
environment = default_environment()
environment["extra"] = ""
for name in INVENTORY["bundled_distributions"]:
    for raw in metadata.requires(name) or ():
        requirement = Requirement(raw)
        if not requirement.marker or requirement.marker.evaluate(environment):
            assert Version(metadata.version(requirement.name)) in requirement.specifier, raw

import six
assert six.ensure_text(b"native") == "native"
from dateutil.parser import isoparse
from dateutil.relativedelta import relativedelta
assert (isoparse("2024-01-31T10:00:00Z") + relativedelta(months=1)).day == 29
from dateutil.zoneinfo import get_zonefile_instance
import datetime
chicago = get_zonefile_instance().zones["America/Chicago"]
assert datetime.datetime(2024, 1, 1, tzinfo=chicago).utcoffset().total_seconds() == -21600
from jinja2 import Environment, DictLoader, select_autoescape
from markupsafe import escape
from markupsafe._speedups import _escape_inner
assert "markupsafe._speedups" in sys.builtin_module_names
assert _escape_inner("<&>") == "&lt;&amp;&gt;"
assert str(escape("<script>")) == "&lt;script&gt;"
template = Environment(loader=DictLoader({"hello": "{{ name }}: {{ values|sum }}"}),
                       autoescape=select_autoescape(default=True))
assert template.get_template("hello").render(name="<world>", values=[20, 22]) == "&lt;world&gt;: 42"
from bs4 import BeautifulSoup
import soupsieve
document = BeautifulSoup("<ul><li class='chosen'>42</li><li>7</li></ul>", "html.parser")
assert soupsieve.select_one("li.chosen", document).get_text() == "42"
from typing_extensions import TypeAliasType
assert TypeAliasType("Numbers", list[int]).__value__ == list[int]
from attrs import define, field, validators, asdict


# ------------------------=
# FUNC: Record
# DESC: Exercises attrs generated constructors, converters, validators and serialization.
# ------------------=
@define
class Record:
    value: int = field(converter=int, validator=validators.ge(0))


assert asdict(Record("42")) == {"value": 42}
try:
    Record(-1)
except ValueError:
    pass
else:
    raise AssertionError("attrs validation failed")
from more_itertools import chunked
assert list(chunked(range(5), 2)) == [[0, 1], [2, 3], [4]]
from toolz import frequencies
assert frequencies("aba") == {"a": 2, "b": 1}
from sortedcontainers import SortedList
ordered = SortedList([3, 1])
ordered.add(2)
assert list(ordered) == [1, 2, 3]
from tabulate import tabulate
assert tabulate([[1, 2]], tablefmt="plain") == "1  2"
import tomli, tomli_w, toml
data = {"title": "Native", "values": [1, 2, 3], "enabled": True}
assert tomli.loads(tomli_w.dumps(data)) == data
assert toml.loads(toml.dumps(data)) == data
from dotenv import dotenv_values
assert dotenv_values(stream=io.StringIO("VALUE=42\nNEXT=${VALUE}\n")) == {"VALUE": "42", "NEXT": "42"}
from colorama import AnsiToWin32
stream = io.StringIO()
AnsiToWin32(stream, strip=True).write("\x1b[31mNative\x1b[0m")
assert stream.getvalue() == "Native"
from pygments import lex
from pygments.lexers import PythonLexer
from pygments.token import Token
assert (Token.Keyword, "return") in list(lex("return 42", PythonLexer()))
import yaml
assert yaml.safe_load(yaml.safe_dump(data)) == data
try:
    yaml.safe_load("!!python/object/apply:os.system ['bad']")
except yaml.constructor.ConstructorError:
    pass
else:
    raise AssertionError("Unsafe YAML object accepted")

# pip's actual metadata consumer must see the entire immutable baseline.
from pip._internal.cli.main import main
from contextlib import redirect_stdout
listing = io.StringIO()
with redirect_stdout(listing):
    assert main(["list", "--format=json", "--disable-pip-version-check"]) == 0
listed = {entry["name"].lower().replace("_", "-"): entry["version"]
          for entry in json.loads(listing.getvalue())}
assert listed == INVENTORY["bundled_distributions"]
assert main(["check", "--disable-pip-version-check"]) == 0
print(json.dumps({"packages_tested": len(INVENTORY["bundled_distributions"]),
                  "native_accelerator": True, "dependencies_satisfied": True}))
