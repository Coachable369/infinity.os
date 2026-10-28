"""Read-only frozen resources and native installation schemes for real pip."""
import io
import os
import sys
from importlib.machinery import FrozenImporter, BuiltinImporter, PathFinder
from importlib.resources.abc import ResourceReader
from _infinity_resources import RESOURCES, DISTRIBUTION_METADATA, PUBLIC_PACKAGE_ROOTS

_original_paths = None


class PublicPackageFinder:
    # ------------------------=
    # FUNC: __init__
    # DESC: Wraps native finders only for public third-party packages, never standard-library or platform modules.
    # ------------------=
    def __init__(self, delegate):
        self.delegate = delegate

    # ------------------------=
    # FUNC: find_spec
    # DESC: Allows authorized user package versions to override the immutable baseline without mixing their submodules.
    # ------------------=
    def find_spec(self, fullname, path=None, target=None):
        if fullname.split(".")[0] in PUBLIC_PACKAGE_ROOTS:
            candidate = PathFinder.find_spec(fullname, path, target)
            if candidate is not None:
                return candidate
            if path is not None and not all(str(item).startswith("/stdlib/") for item in path):
                return None
        return self.delegate.find_spec(fullname, path, target)


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
# FUNC: frozen_resource_data
# DESC: Serves legacy pkgutil package-data reads from verified immutable bytes without exposing host paths.
# ------------------=
def frozen_resource_data(cls, path):
    if not path.startswith("/stdlib/") or ".." in path.split("/"):
        raise FileNotFoundError(path)
    try:
        return RESOURCES[path[len("/stdlib/"):]]
    except KeyError:
        raise FileNotFoundError(path) from None


# ------------------------=
# FUNC: configure
# DESC: Installs frozen resource access and the owned native package scheme before user code runs.
# ------------------=
def configure():
    FrozenImporter.get_resource_reader = classmethod(frozen_resource_reader)
    FrozenImporter.get_data = classmethod(frozen_resource_data)
    sys.meta_path[:] = [PublicPackageFinder(finder) if finder in (BuiltinImporter, FrozenImporter)
                       else finder for finder in sys.meta_path]
    # This is guest VFS scratch, not the host's /tmp. The kernel grants its
    # bounded transaction lifetime and discards it after process completion.
    os.environ["TMPDIR"] = "/tmp"
    os.environ["HOME"] = "/packages"
    import sysconfig
    scheme = dict(sysconfig._INSTALL_SCHEMES["posix_prefix"])
    scheme.update(purelib="{base}", platlib="{base}", scripts="{base}/bin",
                  data="{base}", include="{base}/include", platinclude="{base}/include")
    sysconfig._INSTALL_SCHEMES["infinity"] = scheme
    global _original_paths
    if _original_paths is None:
        _original_paths = sysconfig.get_paths
        sysconfig.get_paths = native_paths
    # pip uses both preferred and default schemes; this is the distributor
    # extension point intentionally exposed by CPython's sysconfig.
    sysconfig._get_preferred_schemes = preferred_schemes
    sysconfig.get_platform = native_platform
    import importlib.metadata

    class FrozenDistribution(importlib.metadata.Distribution):
        # ------------------------=
        # FUNC: __init__
        # DESC: Binds immutable metadata to the exact bundled system distribution.
        # ------------------=
        def __init__(self, metadata):
            self._metadata_files = metadata

        # ------------------------=
        # FUNC: read_text
        # DESC: Exposes genuine installed metadata from the exact pinned upstream wheel.
        # ------------------=
        def read_text(self, filename):
            value = self._metadata_files.get(filename)
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
        # DESC: Discovers bundled distributions only when the immutable system root is in scope, excluding isolated build environments.
        # ------------------=
        @staticmethod
        def find_distributions(context=importlib.metadata.DistributionFinder.Context()):
            name = context.name
            if "/stdlib" not in context.path:
                return
            for installed, metadata in DISTRIBUTION_METADATA.items():
                if name is None or name.lower().replace("_", "-") == installed:
                    yield FrozenDistribution(metadata)

    sys.meta_path.append(FrozenDistributionFinder)
    # Pip continues to own dependency resolution, hashes, metadata and wheel
    # installation. Only transport crosses into the native authenticated actor.
    if "pip" in getattr(sys, "orig_argv", ())[:3]:
        from _infinity_network import configure_pip
        configure_pip()
        from _infinity_subprocess import install
        install()


# ------------------------=
# FUNC: native_paths
# DESC: Honors pip's isolated temporary build prefixes while keeping ordinary installs inside the authorized package root.
# ------------------=
def native_paths(scheme=None, vars=None, expand=True):
    import sysconfig
    scheme = sysconfig.get_default_scheme() if scheme is None else scheme
    if scheme == "infinity" and expand:
        variables = dict(vars or {})
        base = variables.get("base", "/packages")
        if base == sys.prefix:
            base = "/packages"
        base = os.path.normpath(base)
        if base != "/packages" and not base.startswith("/tmp/"):
            raise PermissionError("Python installation prefix is outside its private grants")
        variables["base"] = base
        return _original_paths(scheme, variables, expand)
    return _original_paths(scheme, vars, expand)


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
