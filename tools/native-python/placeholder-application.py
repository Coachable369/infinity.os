"""Deliberately incomplete generated-application fixture; never gameplay evidence."""
from abc import ABC, abstractmethod


class Operation(ABC):
    # ------------------------=
    # FUNC: apply
    # DESC: Declares an intentionally unimplemented abstract operation for the lint fixture.
    # ------------------=
    @abstractmethod
    def apply(self):
        pass


class ConcreteOperation(Operation):
    # ------------------------=
    # FUNC: apply
    # DESC: Preserves a deliberately unimplemented concrete operation for the lint fixture.
    # ------------------=
    def apply(self):
        pass


# ------------------------=
# FUNC: main
# DESC: Provides an intentionally empty entrypoint that generated-only validation must reject.
# ------------------=
def main():
    pass


if __name__ == "__main__":
    main()
