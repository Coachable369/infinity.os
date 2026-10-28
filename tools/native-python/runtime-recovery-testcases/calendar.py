"""Small calendar callback with an inconsistent undated-event sort key."""
from datetime import date


# ------------------------=
# FUNC: on_events_changed
# DESC: Returns events in chronological order with stable ties and undated events last, leaving input unchanged.
# ------------------=
def on_events_changed(events):
    return sorted(events, key=lambda event:
                  date.fromisoformat(event["date"]) if event["date"] else "")


if __name__ == "__main__":
    LAST_RENDERED_EVENTS = on_events_changed([
        {"id": 1, "title": "Follow up", "date": ""},
        {"id": 2, "title": "Meeting", "date": "2026-10-20"},
    ])
    print([event["id"] for event in LAST_RENDERED_EVENTS])
