use super::*;
#[derive(Default)]
struct Trace {
    requests: Vec<HostReadRequest>,
    stop: Option<usize>,
}
impl HostReadBudget for Trace {
    type Error = usize;
    const OBSERVE_ATTEMPTS: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), usize> {
        let n = self.requests.len();
        if self.stop == Some(n) {
            return Err(n);
        }
        self.requests.push(request);
        Ok(())
    }
}

#[test]
fn attempt_bitmap_covers_word_edges_and_all_declaration_orders() {
    for count in [1u32, 63, 64, 65, 127, 128, 129, 8192] {
        for order in 0..3 {
            let mut ids: Vec<_> = (0..count).collect();
            if order == 1 {
                ids.reverse();
            } else if order == 2 {
                ids.rotate_left(count as usize / 3);
            }
            let mut attempts = Attempts::new();
            let mut trace = Trace::default();
            for id in ids {
                let id = ImageResourceId::new(id);
                assert!(!attempts.contains(id, count as usize, &mut trace).unwrap());
                attempts.mark(id, &mut trace).unwrap();
                assert!(attempts.contains(id, count as usize, &mut trace).unwrap());
            }
            let Attempts::Packed {
                words,
                count: bound,
            } = &attempts
            else {
                panic!("bitmap absent")
            };
            assert_eq!(*bound, count as usize);
            assert_eq!(words.len(), (count as usize).div_ceil(64));
            assert_eq!(words.iter().map(|w| w.count_ones()).sum::<u32>(), count);
            assert_eq!(trace.requests.len(), 1 + 3 * count as usize);
            assert_eq!(trace.requests[0].buffer_bytes, words.len() as u64 * 8);
            assert_eq!(trace.requests[0].work, words.len() as u64 * 8 + 64);
            assert!(trace.requests[1..]
                .iter()
                .all(|r| r.work == 64 && r.buffer_bytes == 0));
            let before = trace.requests.len();
            assert!(attempts
                .contains(ImageResourceId::new(count), count as usize, &mut trace)
                .is_err());
            assert_eq!(trace.requests.len(), before);
            assert!(attempts
                .contains(ImageResourceId::new(0), count as usize + 1, &mut trace)
                .is_err());
        }
    }
}

#[test]
fn attempt_migration_refusals_preserve_native_bits_and_retry() {
    let make = || {
        let mut a = Attempts::new();
        for id in [0, 64, 128] {
            a.mark(
                FontFaceId::new(id),
                &mut super::super::resource_read_budget::UnobservedResourceRead,
            )
            .unwrap();
        }
        a
    };
    let mut full = Trace::default();
    assert!(!make()
        .contains(FontFaceId::new(63), 129, &mut full)
        .unwrap());
    assert_eq!(full.requests.len(), 5);
    for stop in 0..full.requests.len() {
        let mut a = make();
        let mut trace = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let ResourceReadError::Budget(failure) = a
            .contains(FontFaceId::new(63), 129, &mut trace)
            .unwrap_err()
        else {
            panic!("wrong refusal")
        };
        assert_eq!(failure.failure.request, full.requests[stop]);
        assert_eq!(
            failure.subject,
            ResourceErrorSubject::FontFace(FontFaceId::new(63))
        );
        assert_eq!(trace.requests, full.requests[..stop]);
        if stop < 4 {
            assert!(matches!(a, Attempts::Native(_)));
        }
        trace.stop = None;
        assert!(!a.contains(FontFaceId::new(63), 129, &mut trace).unwrap());
        for id in [0, 64, 128] {
            assert!(a.contains(FontFaceId::new(id), 129, &mut trace).unwrap());
        }
        a.mark(FontFaceId::new(63), &mut trace).unwrap();
        assert!(a.contains(FontFaceId::new(63), 129, &mut trace).unwrap());
    }
}

#[test]
fn refused_attempt_mark_keeps_bitmap_clear_and_native_calls_reuse_it() {
    let mut a = Attempts::new();
    let id = ImageResourceId::new(64);
    let mut trace = Trace::default();
    assert!(!a.contains(id, 65, &mut trace).unwrap());
    trace.stop = Some(trace.requests.len());
    assert!(a.mark(id, &mut trace).is_err());
    let mut plain = super::super::resource_read_budget::UnobservedResourceRead;
    assert!(!a.contains(id, 65, &mut plain).unwrap());
    a.mark(id, &mut plain).unwrap();
    assert!(a.contains(id, 65, &mut plain).unwrap());
    assert_eq!(trace.requests.len(), 2);
}
