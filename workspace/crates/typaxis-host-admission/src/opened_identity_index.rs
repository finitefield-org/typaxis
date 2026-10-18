//! A fallibly preallocated AVL index; lookup/insert allocate no storage.
use crate::platform::FileIdentity;
use crate::HostAdmissionError;
const NONE: usize = usize::MAX;
#[derive(Clone, Copy, Debug)]
struct Node {
    identity: FileIdentity,
    left: usize,
    right: usize,
    height: u8,
}
pub const HOST_OPEN_IDENTITY_SLOT_BYTES: usize = std::mem::size_of::<Option<FileIdentity>>();
pub const HOST_OPEN_IDENTITY_NODE_BYTES: usize = std::mem::size_of::<Node>();
pub fn host_open_identity_work_bound(attempts: usize) -> Result<u64, HostAdmissionError> {
    let bits = usize::BITS - attempts.leading_zeros();
    u64::from(bits)
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .and_then(|n| n.checked_mul(512))
        .and_then(|n| n.checked_add(1024))
        .ok_or(HostAdmissionError::HostLimit)
}
#[derive(Debug)]
pub(crate) struct Index {
    nodes: Vec<Node>,
    root: usize,
    bound: usize,
}
impl Index {
    pub(crate) fn reserved(capacity: usize) -> Result<Self, HostAdmissionError> {
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(capacity)
            .map_err(|_| HostAdmissionError::ReadCapacity)?;
        Ok(Self {
            nodes,
            root: NONE,
            bound: capacity,
        })
    }
    pub(crate) fn capacity(&self) -> usize {
        self.bound
    }
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }
    pub(crate) fn copy_into(&self, other: &mut Self) {
        assert!(other.capacity() >= self.len());
        other.nodes.extend_from_slice(&self.nodes);
        other.root = self.root;
    }
    fn height(&self, at: usize) -> u8 {
        if at == NONE {
            0
        } else {
            self.nodes[at].height
        }
    }
    fn update(&mut self, at: usize) {
        let n = self.nodes[at];
        self.nodes[at].height = 1 + self.height(n.left).max(self.height(n.right));
    }
    fn left(&mut self, at: usize) -> usize {
        let next = self.nodes[at].right;
        self.nodes[at].right = self.nodes[next].left;
        self.nodes[next].left = at;
        self.update(at);
        self.update(next);
        next
    }
    fn right(&mut self, at: usize) -> usize {
        let next = self.nodes[at].left;
        self.nodes[at].left = self.nodes[next].right;
        self.nodes[next].right = at;
        self.update(at);
        self.update(next);
        next
    }
    fn balance(&mut self, at: usize) -> usize {
        self.update(at);
        let n = self.nodes[at];
        let lh = self.height(n.left);
        let rh = self.height(n.right);
        if lh > rh + 1 {
            let child = self.nodes[n.left];
            if self.height(child.right) > self.height(child.left) {
                self.nodes[at].left = self.left(n.left);
            }
            return self.right(at);
        }
        if rh > lh + 1 {
            let child = self.nodes[n.right];
            if self.height(child.left) > self.height(child.right) {
                self.nodes[at].right = self.right(n.right);
            }
            return self.left(at);
        }
        at
    }
    pub(crate) fn contains(&self, id: FileIdentity) -> bool {
        let mut at = self.root;
        while at != NONE {
            let n = self.nodes[at];
            match id.cmp(&n.identity) {
                std::cmp::Ordering::Equal => return true,
                std::cmp::Ordering::Less => at = n.left,
                std::cmp::Ordering::Greater => at = n.right,
            }
        }
        false
    }
    pub(crate) fn insert(&mut self, id: FileIdentity) -> Result<bool, HostAdmissionError> {
        if self.contains(id) {
            return Ok(false);
        }
        if self.nodes.len() == self.bound {
            return Err(HostAdmissionError::ReadCapacity);
        }
        self.root = self.insert_at(self.root, id);
        Ok(true)
    }
    fn insert_at(&mut self, at: usize, id: FileIdentity) -> usize {
        if at == NONE {
            let next = self.nodes.len();
            self.nodes.push(Node {
                identity: id,
                left: NONE,
                right: NONE,
                height: 1,
            });
            return next;
        }
        if id < self.nodes[at].identity {
            let child = self.insert_at(self.nodes[at].left, id);
            self.nodes[at].left = child;
        } else {
            let child = self.insert_at(self.nodes[at].right, id);
            self.nodes[at].right = child;
        }
        self.balance(at)
    }
}
#[cfg(all(
    test,
    any(target_os = "android", target_os = "linux", target_os = "macos")
))]
mod tests {
    use super::*;
    fn verify(
        t: &Index,
        at: usize,
        low: Option<FileIdentity>,
        high: Option<FileIdentity>,
    ) -> (usize, u8) {
        if at == NONE {
            return (0, 0);
        }
        let n = t.nodes[at];
        assert!(low.map_or(true, |v| v < n.identity) && high.map_or(true, |v| n.identity < v));
        let (a, ah) = verify(t, n.left, low, Some(n.identity));
        let (b, bh) = verify(t, n.right, Some(n.identity), high);
        assert!(ah.abs_diff(bh) <= 1);
        assert_eq!(n.height, 1 + ah.max(bh));
        (a + b + 1, n.height)
    }
    #[test]
    fn avl_index_matches_set_for_forward_reverse_and_permuted_inserts() {
        for order in 0..3 {
            let mut t = Index::reserved(8192).unwrap();
            let mut expected = std::collections::BTreeSet::new();
            for step in 0..8192 {
                let n = match order {
                    0 => step,
                    1 => 8191 - step,
                    _ => (step * 4051) % 8192,
                };
                let id = FileIdentity {
                    device: (n % 7) as u128,
                    inode: n as u128,
                };
                assert_eq!(t.insert(id).unwrap(), expected.insert(id));
                assert!(t.contains(id));
                assert!(!t.insert(id).unwrap());
            }
            let (count, height) = verify(&t, t.root, None, None);
            assert_eq!(count, expected.len());
            assert!(height <= 2 * (usize::BITS - 8192usize.leading_zeros()) as u8);
            for id in expected {
                assert!(t.contains(id));
            }
            let mut copied = Index::reserved(9000).unwrap();
            t.copy_into(&mut copied);
            assert_eq!(verify(&copied, copied.root, None, None).0, 8192);
            assert_eq!(copied.len(), t.len());
        }
    }
}
