"""Read-only frozen resources and native installation schemes for real pip."""
import io
import os
import sys
from importlib.machinery import FrozenImporter
from importlib.resources.abc import ResourceReader
from _infinity_resources import RESOURCES, PIP_METADATA


class FrozenResources(ResourceReader):
    # ------------------------=
    # FUNC: __init__
    # DESC: Binds a reader to a frozen package, never an ambient host path.
    # ------------------=
    def __init__(self, package):
        self.prefix = package.replace(".", "/") + "/"

    # ------------------------=
    # FUNC: open_resource
    # DESC: Returns exact pinned resource bytes using the standard importlib contract.
    # ------------------=
    def open_resource(self, resource):
        if "/" in resource or resource in (".", ".."):
            raise FileNotFoundError(resource)
        try:
            return io.BytesIO(RESOURCES[self.prefix + resource])
        except KeyError:
            raise FileNotFoundError(resource) from None

    # ------------------------=
    # FUNC: resource_path
    # DESC: Signals that frozen resources have no persistent filesystem pathname.
    # ------------------=
    def resource_path(self, resource):
        raise FileNotFoundError(resource)

    # ------------------------=
    # FUNC: is_resource
    # DESC: Tests exact embedded file membership rather than guessing from an extension.
    # ------------------=
    def is_resource(self, name):
        return self.prefix + name in RESOURCES

    # ------------------------=
    # FUNC: contents
    # DESC: Enumerates only direct children of this frozen package's resources.
    # ------------------=
    def contents(self):
        return iter(sorted({path[len(self.prefix):].split("/", 1)[0]
                            for path in RESOURCES if path.startswith(self.prefix)}))


# ------------------------=
# FUNC: frozen_resource_reader
# DESC: Extends the native FrozenImporter with actual pinned package resources.
# ------------------=
def frozen_resource_reader(cls, fullname):
    return FrozenResources(fullname)


# ------------------------=
# FUNC: configure
# DESC: Installs frozen resource access and the owned native package scheme before user code runs.
# ------------------=
def configure():
    FrozenImporter.get_resource_reader = classmethod(frozen_resource_reader)
    # This is guest VFS scratch, not the host's /tmp. The kernel grants its
    # bounded transaction lifetime and discards it after process completion.
    os.environ["TMPDIR"] = "/tmp"
    os.environ["HOME"] = "/packages"
    import sysconfig
    scheme = dict(sysconfig._INSTALL_SCHEMES["posix_prefix"])
    scheme.update(purelib="/packages", platlib="/packages", scripts="/packages/bin",
                  data="/packages", include="/packages/include", platinclude="/packages/include")
    sysconfig._INSTALL_SCHEMES["infinity"] = scheme
    # pip uses both preferred and default schemes; this is the distributor
    # extension point intentionally exposed by CPython's sysconfig.
    sysconfig._get_preferred_schemes = preferred_schemes
    sysconfig.get_platform = native_platform
    import importlib.metadata

    class FrozenDistribution(importlib.metadata.Distribution):
        # ------------------------=
        # FUNC: read_text
        # DESC: Exposes genuine installed pip metadata from the pinned upstream wheel.
        # ------------------=
        def read_text(self, filename):
            value = PIP_METADATA.get(filename)
            return value.decode("utf-8") if value is not None else None

        # ------------------------=
        # FUNC: locate_file
        # DESC: Identifies immutable system distribution paths without granting writable authority.
        # ------------------=
        def locate_file(self, path):
            from pathlib import PurePosixPath
            return PurePosixPath("/stdlib") / path

    class FrozenDistributionFinder:
        # ------------------------=
        # FUNC: find_spec
        # DESC: Leaves module loading to the native FrozenImporter.
        # ------------------=
        @staticmethod
        def find_spec(fullname, path=None, target=None):
            return None

        # ------------------------=
        # FUNC: find_distributions
        # DESC: Discovers bundled pip only when the system distribution root is in scope.
        # ------------------=
        @staticmethod
        def find_distributions(context=importlib.metadata.DistributionFinder.Context()):
            name = context.name
            if "/stdlib" in context.path and (name is None or name.lower().replace("_", "-") == "pip"):
                yield FrozenDistribution()

    sys.meta_path.append(FrozenDistributionFinder)


# ------------------------=
# FUNC: preferred_schemes
# DESC: Keeps all package writes inside the kernel-authorized per-user package mount.
# ------------------=
def preferred_schemes():
    return {"prefix": "infinity", "home": "infinity", "user": "infinity"}


# ------------------------=
# FUNC: native_platform
# DESC: Reports the real InfinityOS target rather than advertising host or Linux wheel compatibility.
# ------------------=
def native_platform():
    return "infinityos_" + sys.implementation._multiarch.removesuffix("-infinityos")
