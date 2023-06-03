use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::{DashMap, DashSet};
use faststr::FastStr;
use fxhash::FxBuildHasher;
use parking_lot::RwLock;

type FxDashMap<K, V> = DashMap<K, V, FxBuildHasher>;
type FxDashSet<K> = DashSet<K, FxBuildHasher>;

#[derive(Debug)]
pub struct Consistent {
    circle: FxDashMap<u32, FastStr>,
    members: FxDashSet<FastStr>,
    sorted_hashes: RwLock<Vec<u32>>,
    number_of_replicas: usize,
    count: AtomicUsize,
}

impl Default for Consistent {
    fn default() -> Self {
        Self::new()
    }
}

impl Consistent {
    pub fn new() -> Self {
        Self {
            circle: FxDashMap::default(),
            members: FxDashSet::default(),
            sorted_hashes: RwLock::new(Vec::new()),
            number_of_replicas: 20,
            count: AtomicUsize::default(),
        }
    }

    pub fn with_number_of_replicas(mut self, number_of_replicas: usize) -> Self {
        self.number_of_replicas = number_of_replicas;
        self
    }

    pub fn add(&self, elt: impl Into<FastStr>) {
        let elt = elt.into();
        for i in 0..self.number_of_replicas {
            self.circle
                .insert(self.hash_key(&elt_key(&elt, i)), elt.clone());
        }
        self.members.insert(elt);
        self.update_sorted_hashes();
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove(&self, elt: impl AsRef<str>) {
        for i in 0..self.number_of_replicas {
            self.circle
                .remove(&self.hash_key(&elt_key(elt.as_ref(), i)));
        }
        self.members.remove(elt.as_ref());
        self.update_sorted_hashes();
        self.count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn set(&self, elts: Vec<impl Into<FastStr>>) {
        let elts = elts.into_iter().map(|elt| elt.into()).collect::<Vec<_>>();
        let mut keys = Vec::with_capacity(self.members().len());
        for member in self.members.iter() {
            let mut found = false;
            for elt in elts.iter() {
                if member.key() == elt {
                    found = true;
                    break;
                }
            }
            if !found {
                keys.push(member.key().to_owned());
            }
        }

        for key in keys {
            self.remove(key);
        }

        for v in elts.into_iter() {
            if !self.members.contains(&v) {
                self.add(v);
            }
        }
    }

    pub fn members(&self) -> Vec<FastStr> {
        self.members
            .iter()
            .map(|member| member.key().to_owned())
            .collect()
    }

    pub fn get(&self, name: impl AsRef<str>) -> Result<FastStr, Error> {
        if self.circle.is_empty() {
            return Err(Error::EmptyCircle);
        }
        let key = self.hash_key(name.as_ref());
        let i = self.search(key);
        Ok(self
            .circle
            .get(&self.sorted_hashes.read()[i])
            .unwrap()
            .to_owned())
    }

    pub fn get_two(&self, name: impl AsRef<str>) -> Result<(FastStr, FastStr), Error> {
        if self.circle.is_empty() {
            return Err(Error::EmptyCircle);
        }
        let key = self.hash_key(name.as_ref());
        let i = self.search(key);
        let a = self
            .circle
            .get(&self.sorted_hashes.read()[i])
            .unwrap()
            .to_owned();
        let mut b = "".into();
        if self.count.load(Ordering::Relaxed) == 1 {
            return Ok((a, b));
        }
        let mut j = i + 1;
        let sorted_hashes = self.sorted_hashes.read();
        while j != i {
            if j >= sorted_hashes.len() {
                j = 0;
            }
            let v = self.circle.get(&sorted_hashes[j]).unwrap();
            if !a.eq(v.value()) {
                b = v.value().to_owned();
                break;
            }
            j += 1;
        }
        Ok((a, b))
    }

    pub fn get_n(&self, name: impl AsRef<str>, mut n: usize) -> Result<Vec<FastStr>, Error> {
        if self.circle.is_empty() {
            return Err(Error::EmptyCircle);
        }
        let count = self.count.load(Ordering::Relaxed);
        if count < n {
            n = count;
        }
        let key = self.hash_key(name.as_ref());
        let i = self.search(key);
        let mut res = Vec::with_capacity(n);
        let sorted_hashes = self.sorted_hashes.read();
        res.push(
            self.circle
                .get(&sorted_hashes[i])
                .unwrap()
                .value()
                .to_owned(),
        );
        if n == 1 {
            return Ok(res);
        }
        let mut j = i + 1;
        while j != i {
            if j >= sorted_hashes.len() {
                j = 0;
            }
            let v = self.circle.get(&sorted_hashes[j]).unwrap();
            if !slice_contains_member(&res, v.value()) {
                res.push(v.value().to_owned());
            }
            if res.len() == n {
                break;
            }
            j += 1;
        }
        Ok(res)
    }

    fn search(&self, key: u32) -> usize {
        let sorted_hashes = self.sorted_hashes.read();
        let i = sorted_hashes.partition_point(|x| *x <= key);
        if i >= sorted_hashes.len() {
            0
        } else {
            i
        }
    }

    fn hash_key(&self, key: &str) -> u32 {
        fxhash::hash32(key)
    }

    fn update_sorted_hashes(&self) {
        let mut sorted_hashes = self.sorted_hashes.write();
        sorted_hashes.clear();

        if sorted_hashes.capacity() / (self.number_of_replicas * 4) > self.circle.len() {
            sorted_hashes.shrink_to(self.circle.len());
        }
        for k in self.circle.iter() {
            sorted_hashes.push(*k.key());
        }
        sorted_hashes.sort();
    }
}

fn elt_key(elt: &str, idx: usize) -> String {
    format!("{}{}", idx, elt)
}

fn slice_contains_member(set: &[FastStr], member: &str) -> bool {
    for m in set.iter() {
        if m == member {
            return true;
        }
    }
    false
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("empty circle")]
    EmptyCircle,
}

#[cfg(test)]
mod tests {
    use crate::Consistent;

    #[test]
    fn test_add() {
        let c = Consistent::new();
        c.add("abcdefg");
        assert_eq!(c.circle.len(), 20);
        assert_eq!(c.sorted_hashes.read().len(), 20);
        c.add("qwer");
        assert_eq!(c.circle.len(), 40);
        assert_eq!(c.sorted_hashes.read().len(), 40);
    }

    #[test]
    fn test_remove() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.remove("abcdefg");
        assert_eq!(c.circle.len(), 0);
        assert_eq!(c.sorted_hashes.read().len(), 0);
    }

    #[test]
    fn test_remove_non_existing() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.remove("abcdefghijk");
        assert_eq!(c.circle.len(), 20);
    }

    #[test]
    fn test_get_empty() {
        let c = Consistent::new();
        let res = c.get("asdfsadfsadf");
        assert!(res.is_err());
    }

    #[test]
    fn test_get_single() {
        let c = Consistent::new();
        c.add("abcdefg");
        let res = c.get("asdfsadfsadf");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "abcdefg");
    }

    #[test]
    fn test_get_two() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.add("opqrstu");
        let res = c.get_two("asdfsadfsadf");
        assert!(res.is_ok());
    }

    #[test]
    fn test_get_n() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.add("opqrstu");
        c.add("hijklmn");
        let res = c.get_n("asdfsadfsadf", 3);
        assert!(res.is_ok());
        let mut res = res.unwrap();
        res.sort();
        assert_eq!(res, vec!["abcdefg", "hijklmn", "opqrstu"]);
    }
    #[test]
    fn test_get_n_more_than_available() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.add("opqrstu");
        c.add("hijklmn");
        let res = c.get_n("asdfsadfsadf", 4);
        assert!(res.is_ok());
        let mut res = res.unwrap();
        res.sort();
        assert_eq!(res, vec!["abcdefg", "hijklmn", "opqrstu"]);
    }

    #[test]
    fn test_get_n_more_than_available_with_repeats() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.add("opqrstu");
        c.add("hijklmn");
        let res = c.get_n("asdfsadfsadf", 5);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 3);
    }

    #[test]
    fn test_set() {
        let c = Consistent::new();
        c.add("abcdefg");
        c.add("opqrstu");
        c.add("hijklmn");
        c.set(vec!["qwer", "asdf"]);
        assert_eq!(c.circle.len(), 40);
        assert_eq!(c.sorted_hashes.read().len(), 40);
    }
}
