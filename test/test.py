from asyncio import gather, get_event_loop, sleep
from contextlib import asynccontextmanager
from os import environ
import subprocess
from time import monotonic
from websockets import connect, ConnectionClosed
from msgpack import loads, dumps
from pytest import mark, raises
from pytest_asyncio import fixture

KiB = 1024
MiB = 1024 * KiB
CHUNK_SIZE = 16 * KiB
sec = 1e9  # ns
SIGKILL = 9
UNSUPPORTED_DATA = 1003
TOO_LARGE = 1009
POLICY_VIOLATION = 1008

url = environ["URL"]

REMOTE = bool(environ.get("REMOTE"))

slow = mark.skipif(bool(environ.get("FAST")), reason="takes too long and FAST option set")


# use only for simple tests which only need one connection and don't handle connection exceptions
@fixture
async def c():
    async with connect(url) as conn:
        yield conn


def req(code, input="", options=(), arguments=(), language="zsh", timeout=60, hook=None):
    d = {
        "language": language,
        "code": code,
        "input": input,
        "arguments": arguments,
        "options": options,
        "timeout": timeout,
    }
    if hook:
        hook(d)
    return dumps(d)


async def test_noop():
    async with connect(url):
        pass


async def test_basic_execution(c):
    await c.send(req(""))
    r = loads(await c.recv())
    assert r.keys() == {"Done"}
    r = r["Done"]
    # check stats values are reasonable
    # assert r["user"] + r["kernel"] <= r["real"]
    assert r.pop("real") < 0.1 * sec
    assert r.pop("kernel") < 0.1 * sec
    assert r.pop("user") < 0.1 * sec
    assert 1000 < r.pop("max_mem") < 100_000  # KiB
    assert 0 <= r.pop("waits") < 1000
    assert 0 <= r.pop("preemptions") < 1000
    assert 0 <= r.pop("major_page_faults") < 1000
    assert 0 <= r.pop("minor_page_faults") < 10000
    assert 0 <= r.pop("input_ops") < 100000
    assert 0 <= r.pop("output_ops") < 100
    assert r == {
        "timed_out": False,
        "stdout_truncated": False,
        "stderr_truncated": False,
        "status_type": "exited",
        "status_value": 0,
    }


async def test_connection_reuse(c):
    await test_basic_execution(c)
    await test_basic_execution(c)


@slow
async def test_code(c):
    start = monotonic()
    await c.send(req("sleep 1"))
    assert loads(await c.recv()).keys() == {"Done"}
    assert REMOTE or 1 < monotonic() - start < 1.1


@slow
async def test_parallelism():
    async def inner():
        async with connect(url) as c:
            await test_code(c)

    start = monotonic()
    await gather(inner(), inner())
    assert REMOTE or 1 < monotonic() - start < 1.2


async def test_stdout(c):
    await c.send(req("echo hello"))
    assert loads(await c.recv()) == {"Stdout": b"hello\n"}
    assert loads(await c.recv()).keys() == {"Done"}


async def test_stderr(c):
    await c.send(req("echo hello >&2"))
    assert loads(await c.recv()) == {"Stderr": b"hello\n"}
    assert loads(await c.recv()).keys() == {"Done"}


async def test_stdin(c):
    await c.send(req("rev", input="hello"))
    assert loads(await c.recv()) == {"Stdout": b"olleh"}
    assert loads(await c.recv()).keys() == {"Done"}


async def test_args(c):
    await c.send(req("echo $@", arguments=["foo", "bar"]))
    assert loads(await c.recv()) == {"Stdout": b"foo bar\n"}
    assert loads(await c.recv()).keys() == {"Done"}


async def test_options(c):
    await c.send(req("echo $-", options=["-F"]))
    assert b"F" in loads(await c.recv())["Stdout"]
    assert loads(await c.recv()).keys() == {"Done"}


async def test_exit(c):
    await c.send(req("exit 7"))
    r = loads(await c.recv())["Done"]
    assert r["status_type"] == "exited"
    assert r["status_value"] == 7


@slow
async def test_timeout(c):
    start = monotonic()
    await c.send(req("sleep 3", timeout=1))
    r = loads(await c.recv())["Done"]
    assert REMOTE or 1 < monotonic() - start < 1.1
    assert r["timed_out"]
    assert r["status_type"] == "killed"
    assert r["status_value"] == SIGKILL


@asynccontextmanager
async def _test_error(msg, code=POLICY_VIOLATION, max_time=0.1):
    start = monotonic()
    with raises(ConnectionClosed) as e:
        async with connect(url) as c:
            yield c
            await c.recv()
    assert REMOTE or monotonic() - start < max_time
    assert e.value.code == code
    assert e.value.reason == msg


@mark.parametrize("kwargs,msg", (
    ({"timeout": 61}, "invalid request: timeout not in range 1-60: 61"),
    ({"timeout": 0}, "invalid request: timeout not in range 1-60: 0"),
    ({"timeout": -4}, "invalid request: timeout not in range 1-60: -4"),
    ({"language": "doesntexist"}, "invalid request: no such language: doesntexist"),
    ({"language": "ZSH"}, "invalid request: no such language: ZSH"),
    ({"arguments": ["null\0byte"]}, "invalid request: argument contains null byte"),
    ({"options": ["null\0byte"]}, "invalid request: argument contains null byte"),
))
async def test_invalid_request_values(kwargs, msg):
    async with _test_error(msg) as c:
        await c.send(req("sleep 1", **kwargs))


class StartsWith(str):
    def __eq__(self, other):
        if isinstance(other, str):
            return other.startswith(self)
        else:
            return NotImplemented


@mark.parametrize("kwargs", (
    {"timeout": "60"},
    {"timeout": 60.0},
    {"timeout": None},
    {"timeout": [60]},
))
async def test_invalid_request_types(kwargs):
    async with _test_error(StartsWith("invalid request:")) as c:
        await c.send(req("sleep 1", **kwargs))


async def test_invalid_request_syntax():
    async with _test_error(StartsWith("invalid request:")) as c:
        await c.send(b"not a valid msgpack message!")


async def test_incomplete_request():
    payload = req("echo hello")
    async with _test_error(StartsWith("invalid request:")) as c:
        await c.send(payload[:20])


async def test_split_request_delay():
    payload = req("echo hello")
    async with _test_error(StartsWith("invalid request:"), max_time=1.1) as c:
        await c.send(payload[:20])
        await sleep(1)
        await c.send(payload[20:])


async def test_split_request():
    payload = req("echo hello")
    async with _test_error(StartsWith("invalid request:")) as c:
        await c.send(payload[:20])
        await c.send(payload[20:])


async def test_invalid_request_data_type():
    async with _test_error("expected a binary message", UNSUPPORTED_DATA) as c:
        await c.send("not a binary message!")


async def test_extra_junk_after_request():
    async with _test_error("invalid request: found extra data") as c:
        await c.send(req("") + b"extra junk")


async def test_too_large_request():
    s = 64 * KiB

    async with _test_error(StartsWith("invalid request:")) as c:
        await c.send(bytes(s))

    async with _test_error(f"received message of size {s + 1}, greater than size limit {s}", TOO_LARGE) as c:
        await c.send(bytes(s + 1))


@mark.parametrize("kwargs", (
    {"input": "unicode"},
    {"input": b"bytes"},
    {"input": list(b"bytes")},
    {"hook": lambda d: d.pop("timeout")},
))
async def test_valid_request_types(c, kwargs):
    await c.send(req("", **kwargs))
    assert "Done" in loads(await c.recv())


@slow
async def test_streaming(c):
    then = monotonic()
    await c.send(req("repeat 3 sleep 1 && echo hi "))
    for _ in range(3):
        assert loads(await c.recv())["Stdout"] == b"hi\n"
        now = monotonic()
        assert 0.9 < now - then < 1.1
        then = now
    assert "Done" in loads(await c.recv())
    assert REMOTE or monotonic() - then < 0.1


@slow
async def test_kill(c):
    await c.send(req("sleep 3"))
    await sleep(1)
    start = monotonic()
    await c.send(dumps("Kill"))
    r = loads(await c.recv())["Done"]
    assert REMOTE or monotonic() - start < 0.1
    assert r["status_type"] == "killed"
    assert r["status_value"] == SIGKILL


@slow
@mark.parametrize("close", ["0<&-", ">&-", "2>&-"])
async def test_close_stdio(c, close):
    start = monotonic()
    await c.send(req(f"exec {close}; sleep 1"))
    assert "Done" in loads(await c.recv())
    assert REMOTE or 1 < monotonic() - start < 1.1


async def pgrep(*args):
    assert not REMOTE
    def inner():
        proc = subprocess.run(["pgrep", *args])
        match proc.returncode:
            case 0:
                return True
            case 1:
                return False
            case _:
                raise RuntimeError("pgrep failed", proc.stderr)
    return await get_event_loop().run_in_executor(None, inner)


@slow
async def test_client_close():
    start = monotonic()
    async with connect(url) as c:
        await c.send(req("sleep 5"))
        await sleep(1)
        assert REMOTE or await pgrep("sleep")
    await sleep(1)
    assert REMOTE or monotonic() - start < 2.1
    assert REMOTE or not await pgrep("sleep")


@slow
async def test_client_close_yes():
    async with connect(url) as c:
        await c.send(req("yes"))
    await sleep(1)
    assert REMOTE or not await pgrep("yes")


async def test_large_output(c):
    await c.send(req(f"dd if=/dev/zero bs={CHUNK_SIZE * 3} count=1 2>&-"))
    for _ in range(3):
        assert loads(await c.recv()) == {"Stdout": bytes(CHUNK_SIZE)}
    assert "Done" in loads(await c.recv())


@mark.parametrize("name", ["stdout", "stderr"])
async def test_truncated(c, name):
    start = monotonic()
    n = CHUNK_SIZE * 1000
    SIZE_LIMIT = 128 * KiB
    assert n > SIZE_LIMIT
    await c.send(req(f"(dd if=/dev/zero bs={n} count=1 2>&-) >/dev/{name}"))
    for _ in range(SIZE_LIMIT // CHUNK_SIZE + 1):
        r = loads(await c.recv())
        print(len(r[[*r][0]]))
        assert r == {name.capitalize(): bytes(CHUNK_SIZE)}
    r = loads(await c.recv())["Done"]
    assert r[f"{name}_truncated"] is True
    assert REMOTE or monotonic() - start < 0.5


async def test_yes():
    start = monotonic()
    async with connect(url) as c:
        await c.send(req("yes"))
        async for msg in c:
            if "Done" in loads(msg):
                break
    assert REMOTE or monotonic() - start < 1
    assert REMOTE or not await pgrep("yes")


async def test_yes_kill():
    async with connect(url) as c:
        await c.send(req("yes"))
        for _ in range(3):
            assert "Stdout" in loads(await c.recv())
        start = monotonic()
        await c.send(dumps("Kill"))
        async for msg in c:
            if "Done" in loads(msg):
                break
    assert REMOTE or monotonic() - start < 1
    assert REMOTE or not await pgrep("yes")


async def test_loopback(c):
    await c.send(req("ip addr"))
    assert loads(await c.recv())["Stdout"] == b"""\
1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
    inet 127.0.0.1/8 scope host lo
       valid_lft forever preferred_lft forever
    inet6 ::1/128 scope host 
       valid_lft forever preferred_lft forever
"""


async def test_tmp(c):
    await c.send(req("touch /tmp/foo; ls /tmp"))
    assert loads(await c.recv())["Stdout"] == b"foo\n"


async def test_writeable_rootfs(c):
    await c.send(req("echo hi > foo; cat foo"))
    assert loads(await c.recv())["Stdout"] == b"hi\n"


async def test_writeable_rootfs_everywhere(c):
    await c.send(req("echo hi > /etc/shadow; cat /etc/shadow"))
    assert loads(await c.recv())["Stdout"] == b"hi\n"
