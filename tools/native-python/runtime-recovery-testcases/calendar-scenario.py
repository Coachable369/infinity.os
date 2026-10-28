"""Trusted date/order checks, kept separate from model-editable callback source."""
MINIMUM_CHECKS = 12


# ------------------------=
# FUNC: verify
# DESC: Checks date ordering, stable ties, undated placement, nonmutation and invalid dates using actual callback outputs.
# ------------------=
def verify(candidate, check):
    check(candidate["__name__"] == "__main__")
    changed = candidate["on_events_changed"]
    check(callable(changed))
    check([event["id"] for event in candidate["LAST_RENDERED_EVENTS"]] == [2, 1])
    check(changed([]) == [])
    events = [
        {"id": 1, "title": "Unscheduled", "date": ""},
        {"id": 2, "title": "Later", "date": "2027-01-01"},
        {"id": 3, "title": "Earlier", "date": "2026-12-31"},
        {"id": 4, "title": "Same date", "date": "2026-12-31"},
        {"id": 5, "title": "Also unscheduled", "date": ""},
    ]
    before = [dict(event) for event in events]
    ordered = changed(events)
    check([event["id"] for event in ordered] == [3, 4, 2, 1, 5])
    check(events == before)
    check(ordered is not events)
    check(len(ordered) == len(events))
    check({event["id"]: event for event in ordered} == {event["id"]: event for event in events})
    check(changed([events[0], events[4]]) == [events[0], events[4]])
    check(changed([events[2]]) == [events[2]])
    try:
        changed([{"id": 9, "title": "Invalid", "date": "2026-02-30"}])
    except ValueError:
        check(True)
    else:
        check(False)
