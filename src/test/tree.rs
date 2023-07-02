use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{Arc, Weak},
};

use anyhow::Context;
use derive_more::From;
use trait_set::trait_set;

use crate::utils::mutex::Mutex;

use super::result::{TestError, TestResult};

trait_set! {
    pub trait OnCompleteCallback<C> = Fn(&GenericTestNode<C>, &TestResult) + Send + Sync;
}

pub struct GenericTestNode<C> {
    parent: Option<Weak<ParentTestNode>>,
    name: Cow<'static, str>,
    full_name: String,
    content: C,
    pub result: Mutex<Option<TestResult>>,
    on_complete: Option<Box<dyn OnCompleteCallback<C>>>,
}

pub type ParentTestNode = GenericTestNode<Mutex<ParentNodeContent>>;
pub type LeafTestNode = GenericTestNode<()>;

#[derive(From)]
pub enum TestNode {
    Parent(Arc<ParentTestNode>),
    Leaf(Arc<LeafTestNode>),
}

#[derive(Default)]
pub struct ParentNodeContent {
    children: BTreeMap<Cow<'static, str>, TestNode>,
}

impl ParentTestNode {
    pub fn new_root<F>(name: impl Into<Cow<'static, str>>, on_complete: F) -> Arc<Self>
    where
        F: OnCompleteCallback<Mutex<ParentNodeContent>> + 'static,
    {
        let name = name.into();
        Arc::new(Self {
            name: name.clone(),
            full_name: String::from(name),
            content: Mutex::new(ParentNodeContent::default()),
            on_complete: Some(Box::new(on_complete)),
            parent: None,
            result: Mutex::new(None),
        })
    }

    fn new_child<C>(&self, child: GenericTestNode<C>) -> Arc<GenericTestNode<C>>
    where
        TestNode: From<Arc<GenericTestNode<C>>>,
    {
        let child = Arc::new(child);
        let mut content = self.content.lock();
        let ret_child = child.clone();
        let old_value = content
            .children
            .insert(child.name.clone(), TestNode::from(child));
        debug_assert!(old_value.is_none());
        *self.result.lock() = None;
        ret_child
    }

    pub fn new_child_parent(
        self: &Arc<Self>,
        name: impl Into<Cow<'static, str>>,
    ) -> Arc<ParentTestNode> {
        let name = name.into();
        self.new_child(Self {
            parent: Some(Arc::downgrade(self)),
            full_name: format!("{}.{}", self.full_name, name),
            name,
            result: Mutex::new(None),
            content: Mutex::new(ParentNodeContent::default()),
            on_complete: None,
        })
    }

    pub fn new_child_leaf(
        self: &Arc<Self>,
        name: impl Into<Cow<'static, str>>,
    ) -> Arc<LeafTestNode> {
        let name = name.into();
        self.new_child(GenericTestNode {
            parent: Some(Arc::downgrade(self)),
            full_name: format!("{}.{}", self.full_name, name),
            name,
            result: Mutex::new(None),
            content: (),
            on_complete: None,
        })
    }

    fn update_child(&self, name: &str, new_result: TestResult) {
        {
            let lock = self.content.lock();
            let child = lock
                .children
                .get(name)
                .unwrap_or_else(|| panic!("child test node named {name} not found"));
            match child {
                TestNode::Parent(par) => *par.result.lock() = Some(new_result),
                TestNode::Leaf(leaf) => *leaf.result.lock() = Some(new_result),
            }
        }

        if let Some(result) = self.get_result() {
            self.update_result(result);
        }
    }

    fn get_result(&self) -> Option<TestResult> {
        let lock = self.content.lock();
        let mut failed_tests = Vec::new();
        let mut pending_tests = Vec::new();
        for (name, node) in lock.children.iter() {
            let (guard, full_name) = match node {
                TestNode::Parent(par) => (par.result.lock(), par.full_name.clone()),
                TestNode::Leaf(leaf) => (leaf.result.lock(), leaf.full_name.clone()),
            };

            match *guard {
                Some(TestResult::Err(_)) => failed_tests.push(full_name.into()),
                None => pending_tests.push(name.clone()),
                _ => {}
            }
        }

        if pending_tests.is_empty() {
            if failed_tests.is_empty() {
                Some(TestResult::Ok(()))
            } else {
                Some(TestResult::Err(TestError::ChildFailedError(failed_tests)))
            }
        } else {
            None
        }
    }
}

impl LeafTestNode {
    pub fn update(&self, result: TestResult) {
        tracing::info!(
            "test `{}` finished with result {:?}",
            self.full_name,
            result
        );
        debug_assert!(self.parent.is_some());
        self.update_result(result);
    }
}

impl<C> GenericTestNode<C> {
    fn update_result(&self, result: TestResult) {
        if let Some(on_complete) = self.on_complete.as_ref() {
            (on_complete)(self, &result);
        }

        if let Some(parent) = self.parent.as_ref() {
            if let Ok(parent) = parent.upgrade().context("parent node was dropped") {
                parent.update_child(&self.name, result);
            }
        }
    }

    pub fn finished(&self) -> bool {
        self.result.lock().is_some()
    }

    pub fn full_name(&self) -> &str {
        self.full_name.as_str()
    }
}
