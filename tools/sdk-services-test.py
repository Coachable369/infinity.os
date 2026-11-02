"""Behavioral tests of public SDK bindings; no live host network is contacted."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: module
# DESC: Loads a public Python SDK module without requiring native GUI services on the build host.
# ------------------=
def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "sdk/python" / (name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result

# ------------------------=
# FUNC: main
# DESC: Checks cross-language JSON records, native wrapper contracts and strict TLS cleanup behavior.
# ------------------=
def main():
    c_record = json.loads(subprocess.check_output([str(ROOT / "build/tests/sdk-services-test")]))
    logging = module("infinity_log")
    sink = io.StringIO()
    logging.log(logging.INFO, "demo", 'line\n"quoted"\\end', sink)
    assert json.loads(sink.getvalue()) == c_record
    assert len(sink.getvalue().splitlines()) == 1
    for args in ((4,"x","x"),(1,"x"*33,"x"),(1,"x","x"*257)):
        try:
            logging.log(*args, sink=sink)
        except ValueError:
            pass
        else:
            raise AssertionError(args)
    broken = Mock(); broken.write.return_value = 0
    try:
        logging.log(1,"x","x",broken)
    except OSError:
        pass
    else:
        raise AssertionError("short write accepted")
    network = module("infinity_network")
    stream, encrypted, context = Mock(), Mock(), Mock()
    context.wrap_socket.return_value = encrypted
    with patch.object(network.socket,"create_connection",return_value=stream) as create, patch.object(network.ssl,"create_default_context",return_value=context):
        assert network.connect("example.com",443,tls=True) is encrypted
        create.assert_called_once_with(("example.com",443),10.0)
        context.wrap_socket.assert_called_once_with(stream,server_hostname="example.com")
        context.wrap_socket.side_effect = OSError("certificate rejected")
        try:
            network.connect("example.com",443,tls=True)
        except OSError:
            pass
        else:
            raise AssertionError("TLS failure hidden")
        stream.close.assert_called_once()
        for port, timeout in ((0,10),(65536,10),(443,0),(443,121)):
            before=create.call_count
            try:
                network.connect("example.com",port,timeout)
            except ValueError:
                pass
            else:
                raise AssertionError("invalid endpoint accepted")
            assert create.call_count == before
    print("SDK service wrappers: structured records, bounds, errors, TLS cleanup and ABI checks passed")

if __name__ == "__main__":
    main()
