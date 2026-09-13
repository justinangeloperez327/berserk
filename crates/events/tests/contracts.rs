use framework_events::{DispatchError, ErrorKind, Event, EventBus};
use std::sync::{Arc, Mutex};

struct Created {
    id: u64,
}
impl Event for Created {
    const NAME: &'static str = "created";
}

#[test]
fn listeners_run_in_registration_order() {
    let bus = EventBus::new();
    let calls = Arc::new(Mutex::new(Vec::new()));
    for number in [1, 2] {
        let calls = Arc::clone(&calls);
        bus.listen::<Created, _>(move |event| {
            calls.lock().unwrap().push((number, event.id));
            Ok(())
        })
        .unwrap();
    }
    assert_eq!(bus.dispatch(&Created { id: 9 }).unwrap().listeners_run, 2);
    assert_eq!(*calls.lock().unwrap(), vec![(1, 9), (2, 9)]);
}

#[test]
fn removal_and_failure_are_explicit() {
    let bus = EventBus::new();
    let removed = bus.listen::<Created, _>(|_| Ok(())).unwrap();
    assert!(bus.forget::<Created>(removed).unwrap());
    bus.listen::<Created, _>(|_| {
        Err(DispatchError::new(
            ErrorKind::Listener,
            "wrong",
            None,
            "stop",
        ))
    })
    .unwrap();
    let error = bus.dispatch(&Created { id: 1 }).unwrap_err();
    assert_eq!(error.message(), "stop");
    assert_eq!(error.kind(), ErrorKind::Listener);
    assert_eq!(error.event(), Created::NAME);
}

#[test]
fn listener_panics_are_contained() {
    let bus = EventBus::new();
    bus.listen::<Created, _>(|_| -> framework_events::Result<()> { panic!("listener panic") })
        .unwrap();
    assert_eq!(
        bus.dispatch(&Created { id: 1 }).unwrap_err().kind(),
        ErrorKind::ListenerPanic
    );
}
