//! Stateless UIKit interaction contracts. Services retain values and authority;
//! controls propose changes and never treat a painted state as committed state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Checkbox(bool),
    Radio { selected: usize, count: usize },
    Choice { selected: usize, count: usize },
    Expander(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input { Activate, Next, Previous, Select(usize), Collapse }

pub const SETTINGS_ROW_FOCUS: usize = 128;

// ------------------------=
// FUNC: settings_tab_focus
// DESC: Traverses a Settings section's row controls and returns to its sidebar at either boundary, without changing any setting value.
// ------------------=
pub fn settings_tab_focus(focus: usize, rows: usize, backwards: bool) -> usize {
    if rows == 0 || rows > 128 { return 0; }
    let row = focus.checked_sub(SETTINGS_ROW_FOCUS).filter(|&row|row<rows);
    match (row,backwards) {
        (None,false) => SETTINGS_ROW_FOCUS,
        (None,true) => SETTINGS_ROW_FOCUS+rows-1,
        (Some(0),true) => 0,
        (Some(row),true) => SETTINGS_ROW_FOCUS+row-1,
        (Some(row),false) if row+1<rows => SETTINGS_ROW_FOCUS+row+1,
        _ => 0,
    }
}

// ------------------------=
// FUNC: transition
// DESC: Computes one bounded, disabled-aware control change for mouse, keyboard or accessibility activation without executing a service action.
// ------------------=
pub fn transition(value: Value, input: Input, enabled: bool) -> Option<Value> {
    if !enabled { return None; }
    let result = match (value, input) {
        (Value::Checkbox(on), Input::Activate) => Value::Checkbox(!on),
        (Value::Expander(open), Input::Activate) => Value::Expander(!open),
        (Value::Expander(_), Input::Collapse) => Value::Expander(false),
        (Value::Radio { selected, count }, event) | (Value::Choice { selected, count }, event) => {
            if count == 0 || selected >= count { return None; }
            let next = match event {
                Input::Activate | Input::Next => if selected + 1 == count { 0 } else { selected + 1 },
                Input::Previous => if selected == 0 { count - 1 } else { selected - 1 },
                Input::Select(index) if index < count => index,
                _ => return None,
            };
            match value {
                Value::Radio { .. } => Value::Radio { selected: next, count },
                _ => Value::Choice { selected: next, count },
            }
        }
        _ => return None,
    };
    (result != value).then_some(result)
}

// ------------------------=
// FUNC: next_choice
// DESC: Resolves a circular selector through the same bounded interaction contract used by UIKit controls.
// ------------------=
pub fn next_choice(selected: usize, count: usize) -> Option<usize> {
    match transition(Value::Choice { selected, count }, Input::Activate, true)? {
        Value::Choice { selected, .. } => Some(selected), _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: activation_and_bounds
    // DESC: Exercises each default control's actual state transitions, disabled handling, radio exclusivity, circular selection and invalid input rejection.
    // ------------------=
    #[test]
    fn activation_and_bounds() {
        assert_eq!(transition(Value::Checkbox(false), Input::Activate, true), Some(Value::Checkbox(true)));
        assert_eq!(transition(Value::Checkbox(true), Input::Activate, false), None);
        assert_eq!(transition(Value::Expander(true), Input::Collapse, true), Some(Value::Expander(false)));
        assert_eq!(transition(Value::Expander(false), Input::Collapse, true), None);
        for count in 1..=32 {
            let mut selected = 0;
            for step in 1..=count * 3 {
                selected = next_choice(selected, count).unwrap_or(selected);
                assert_eq!(selected, step % count);
            }
            let radio = Value::Radio { selected: 0, count };
            assert_eq!(transition(radio, Input::Select(count), true), None);
            assert_eq!(transition(radio, Input::Select(0), true), None);
            if count > 1 {
                assert_eq!(transition(radio, Input::Previous, true), Some(Value::Radio { selected: count - 1, count }));
                assert_eq!(transition(radio, Input::Select(count - 1), true), Some(Value::Radio { selected: count - 1, count }));
            }
        }
        assert_eq!(next_choice(0, 0), None);
        assert_eq!(next_choice(5, 5), None);
        for rows in 1..=128 {
            let mut focus=0;
            for row in 0..rows {
                focus=settings_tab_focus(focus,rows,false);
                assert_eq!(focus,SETTINGS_ROW_FOCUS+row);
            }
            assert_eq!(settings_tab_focus(focus,rows,false),0);
            for row in (0..rows).rev() {
                focus=settings_tab_focus(if row==rows-1 {0}else{focus},rows,true);
                assert_eq!(focus,SETTINGS_ROW_FOCUS+row);
            }
            assert_eq!(settings_tab_focus(focus,rows,true),0);
        }
    }
}
