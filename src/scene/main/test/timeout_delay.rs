use std::{
    ops::Sub,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;

use crate::{
    exec::main_ctx::MainContext,
    test::{
        assert::{assert_greater_equals, assert_less_equals},
        result::TestResult,
        tree::ParentTestNode,
    },
};

const MAX_DELAY: Duration = Duration::from_millis(100);

pub fn test(main_ctx: &mut MainContext, node: &Arc<ParentTestNode>) -> anyhow::Result<()> {
    let node = node.new_child_parent("set_timeout_delay");

    let mut test = |timeout: Duration, name: &'static str| -> anyhow::Result<()> {
        let test_node = node.new_child_leaf(name);
        let now = Instant::now();

        fn do_test(elapsed: Duration, timeout: Duration) -> TestResult {
            assert_greater_equals(&elapsed, &timeout, "elapsed must be greater than timeout")?;
            let delay = elapsed.sub(timeout);
            assert_less_equals(&delay, &MAX_DELAY, "more timeout delay than expected")?;
            Ok(())
        }

        main_ctx
            .set_timeout(timeout, move |_, _| {
                test_node.update(do_test(now.elapsed(), timeout));
                Ok(())
            })
            .context("unable to set timeout")?;
        Ok(())
    };

    test(Duration::from_millis(100), "100ms")?;
    test(Duration::from_secs(1), "1s")?;
    test(Duration::from_millis(1500), "1.5s")?;
    test(Duration::from_secs(3), "3s")?;
    test(Duration::from_secs(5), "5s")?;
    test(Duration::from_secs(10), "10s")?;
    Ok(())
}
