use soroban_sdk::Env;
use colored::Colorize;

/// A recorded sequence of calls that can be replayed for regression checking.
///
/// # Example
/// ```ignore
/// let mut seq = CallSequence::new(&env);
/// seq.call("mint(alice, 1000)", || token_client.mint(&alice, &1000_i128));
/// seq.call("transfer(alice→bob, 500)", || token_client.transfer(&alice, &bob, &500_i128));
/// seq.replay();
/// ```
pub struct CallSequence {
    env: Env,
    calls: Vec<(String, Box<dyn Fn() + 'static>)>,
}

impl CallSequence {
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            calls: Vec::new(),
        }
    }

    pub fn call(&mut self, label: impl Into<String>, action: impl Fn() + 'static) -> &mut Self {
        self.calls.push((label.into(), Box::new(action)));
        self
    }

    /// Replay all recorded calls in order. Panics on any failure.
    pub fn replay(&self) {
        println!(
            "\n{} Replaying {} calls",
            "↺".cyan().bold(),
            self.calls.len()
        );
        for (label, action) in &self.calls {
            self.env.budget().reset_default();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action()));
            let cpu = self.env.budget().cpu_instruction_cost();
            match result {
                Ok(_) => println!(
                    "  {} {}  [cpu={}]",
                    "✓".green(),
                    label,
                    cpu.to_string().cyan()
                ),
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    panic!("anvil: replay failed at '{}': {}", label, msg);
                }
            }
        }
        println!("{} Replay complete\n", "✓".green().bold());
    }

    /// Run calls in every possible permutation and assert all succeed.
    /// Only practical for small sequences (≤6 calls) due to n! growth.
    pub fn assert_order_independent(&self) {
        let n = self.calls.len();
        assert!(
            n <= 6,
            "assert_order_independent: too many calls ({}). Max is 6 to keep n! manageable.",
            n
        );

        let indices: Vec<usize> = (0..n).collect();
        let perms = Self::permutations(indices);
        let total = perms.len();

        println!(
            "\n{} Testing {} permutations of {} calls...",
            "⚡".yellow().bold(),
            total,
            n
        );

        let mut failures = 0usize;
        for perm in &perms {
            for &i in perm {
                let (_, action) = &self.calls[i];
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action()));
                if result.is_err() {
                    failures += 1;
                }
            }
        }

        if failures == 0 {
            println!("{} All {} permutations passed", "✓".green().bold(), total);
        } else {
            panic!(
                "anvil: {}/{} permutations failed — calls are order-dependent",
                failures,
                total
            );
        }
    }

    fn permutations(mut items: Vec<usize>) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        Self::heap_perm(&mut items, items.len(), &mut result);
        result
    }

    fn heap_perm(arr: &mut Vec<usize>, n: usize, result: &mut Vec<Vec<usize>>) {
        if n == 1 {
            result.push(arr.clone());
            return;
        }
        for i in 0..n {
            Self::heap_perm(arr, n - 1, result);
            if n % 2 == 0 {
                arr.swap(i, n - 1);
            } else {
                arr.swap(0, n - 1);
            }
        }
    }
}
