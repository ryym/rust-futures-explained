#![allow(clippy::mutex_atomic)]

// https://cfsamson.github.io/books-futures-explained/6_future_example.html#bonus-section---a-proper-way-to-park-our-thread
// 単純に thread の park/unpark を使って Future の pending/wake を実装すると問題がある。
// 例えば Future とは無関係な箇所で park/unpark を使う処理があると、処理の順序次第では
// Waker の unpark が Future とは別箇所の park を解除する形になり、その後に Future の park が走ると
// そちらは解除されず残り続ける事になる。
// 例 (ログを追加してわかりやすくした):
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=4df8466eee5333afe3d061e7a9b73691
//
// なので thread の park ではなく専用のブロック処理を実装する必要がある。
// Condvar を使うと以下のような Parker を実装できる。

use std::sync::{Condvar, Mutex};

#[derive(Default, Debug)]
pub struct Parker {
    mutex: Mutex<bool>,
    cond: Condvar,
}

impl Parker {
    pub fn park(&self) {
        let mut resumable = self.mutex.lock().unwrap();
        resumable = self.cond.wait_while(resumable, |r| !*r).unwrap();
        *resumable = false;
    }

    pub fn unpark(&self) {
        *self.mutex.lock().unwrap() = true;
        self.cond.notify_one();
    }
}
