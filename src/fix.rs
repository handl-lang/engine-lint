//! Byte-splice fix application for lint diagnostics (PL-004 T-091).
//!
//! Lint fixes are *byte-range splices* — [`LintFixOut`] `{ start, end,
//! replacement }` — not AST rewrites. They are produced by `!Native`
//! checks (scope/naming/source rules, `.hnl` handl natives) and by
//! `!Match` rules' declarative `!Unwrap` fix alike, so this module owns
//! the pieces that are lint-domain knowledge:
//!
//! - [`fix_batch`] — which fixes of one lint pass apply together
//!   (the whole-file-fix-subsumes-others policy, the `source.fmt`
//!   precedent generalized);
//! - [`apply_fix_batch`] — the descending byte splice itself, over
//!   `plugin-ext-diag`'s `apply_fixes`;
//! - [`fixpoint`] / [`LintEngine::fix_source`](crate::LintEngine::fix_source)
//!   — lint → fix → re-lint until no fixes remain or the iteration
//!   budget is spent.
//!
//! What this module deliberately does *not* do: turn fixes into
//! `Change`s or write files. The converged fix set is legitimate
//! desired-state — `hnl-tool-codemod`'s
//! [`CodemodEngine::plan_byte_edits`](https://docs.rs/hnl-tool-codemod)
//! accepts each [`FixHit`] mapped to a `ByteEdit` and emits journaled,
//! revertable `Change::Put`s through `hnl-tool-codegen`. That wiring
//! (and the `hnl lint --fix` command path) is plugin-side.

use std::fmt;

use handl_language::LangError;
use handl_plugin_ext_diag::{Fix, FixError, Span as DiagSpan, apply_fixes};

use crate::{LintDiagnostic, LintEngine, LintFixOut};

/// A lint fix plus the name of the rule that produced it.
///
/// `rule` is provenance carried through to the codemod engine's
/// `ByteEdit::rule` (→ `CodemodEdit::rule_id`); `LintFixOut` itself has
/// no rule field — the rule lives on the [`LintDiagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixHit {
    /// The lint rule name (`LintDiagnostic::rule`).
    pub rule: String,
    /// The byte-range fix.
    pub fix: LintFixOut,
}

/// Errors from [`fixpoint`] / [`LintEngine::fix_source`](crate::LintEngine::fix_source).
#[derive(Debug)]
pub enum LintFixError {
    /// Linting failed — the source does not lex/parse, or a rule
    /// errored. Fixes cannot be computed; propagated rather than
    /// silently treated as "no fixes".
    Lint(LangError),
    /// The fix batch failed to apply (overlap or out-of-bounds —
    /// see [`handl_plugin_ext_diag::FixError`]).
    Apply(FixError),
    /// The fixpoint loop did not converge within `max_iterations`.
    NotConverged {
        /// The iteration budget that was exhausted.
        max_iterations: usize,
    },
}

impl fmt::Display for LintFixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lint(e) => write!(f, "lint failed: {e}"),
            Self::Apply(e) => write!(f, "fix application failed: {e}"),
            Self::NotConverged { max_iterations } => {
                write!(
                    f,
                    "fixpoint did not converge within {max_iterations} iterations"
                )
            }
        }
    }
}

impl std::error::Error for LintFixError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lint(e) => Some(e),
            Self::Apply(e) => Some(e),
            Self::NotConverged { .. } => None,
        }
    }
}

/// Select the fix batch for one lint pass over a source of `src_len`
/// bytes.
///
/// Policy (the `plugin-ext-lint` `compute_fixes` precedent,
/// generalized): a fix spanning the *entire* source (`0..src_len`)
/// subsumes every other fix — it overlaps them all by construction
/// (this is how `source.fmt`'s whole-file rewrite fix works). When one
/// is present, only the first is returned. Otherwise every diagnostic
/// fix is returned in diagnostic order and [`apply_fix_batch`] rejects
/// any overlap — a conflicting batch is an error, never silently
/// dropped.
pub fn fix_batch(src_len: usize, diags: &[LintDiagnostic]) -> Vec<FixHit> {
    let hits: Vec<FixHit> = diags
        .iter()
        .filter_map(|d| {
            d.fix.clone().map(|fix| FixHit {
                rule: d.rule.clone(),
                fix,
            })
        })
        .collect();
    if src_len > 0
        && let Some(pos) = hits
            .iter()
            .position(|h| h.fix.start == 0 && h.fix.end == src_len)
    {
        return vec![hits.into_iter().nth(pos).expect("position checked")];
    }
    hits
}

/// Apply a [`fix_batch`] selection to `src`: descending byte-order
/// splice via `plugin-ext-diag`'s [`apply_fixes`] (overlap and
/// out-of-bounds are errors).
///
/// # Errors
///
/// [`FixError::Overlapping`] / [`FixError::OutOfBounds`] from
/// [`apply_fixes`].
pub fn apply_fix_batch(src: &str, hits: &[FixHit]) -> Result<String, FixError> {
    let fixes: Vec<Fix> = hits
        .iter()
        .map(|h| Fix {
            span: DiagSpan::from_byte_range(h.fix.start, h.fix.end, src),
            replacement: h.fix.replacement.clone(),
        })
        .collect();
    apply_fixes(src, &fixes)
}

/// The outcome of a converged (or budget-exhausted) fix run.
#[derive(Debug)]
pub struct FixpointOutcome {
    /// The fixed source text.
    pub source: String,
    /// How many fix rounds were applied.
    pub iterations: usize,
    /// The fixes applied per round, in application order — provenance
    /// for downstream edit records (`ByteEdit`s).
    pub rounds: Vec<Vec<FixHit>>,
    /// Diagnostics from the final lint pass (the pass that produced no
    /// fixes). These are the *remaining* diagnostics — unfixable or
    /// fix-free.
    pub diagnostics: Vec<LintDiagnostic>,
}

/// Run the lint→fix→re-lint fixpoint.
///
/// `lint` computes diagnostics for a source text — typically
/// `|s| engine.lint_source(lang, s, lexer, parser)` (see
/// [`LintEngine::fix_source`]). Each round applies [`fix_batch`] +
/// [`apply_fix_batch`]; the loop ends when a pass yields no fixes
/// (converged) or `max_iterations` rounds have been applied without
/// convergence ([`LintFixError::NotConverged`]).
///
/// Unlike `plugin-ext-lint`'s `compute_fixes` (which swallowed lint
/// errors into "no fixes"), a lint failure propagates as
/// [`LintFixError::Lint`] — an unparseable intermediate is an honest
/// error, not a silent stop.
///
/// # Errors
///
/// [`LintFixError::Lint`] on the first failing lint pass,
/// [`LintFixError::Apply`] on a bad batch, [`LintFixError::NotConverged`]
/// when the budget is spent with fixes still pending.
pub fn fixpoint<F>(
    source: &str,
    max_iterations: usize,
    lint: F,
) -> Result<FixpointOutcome, LintFixError>
where
    F: Fn(&str) -> Result<Vec<LintDiagnostic>, LangError>,
{
    let mut current = source.to_string();
    let mut rounds: Vec<Vec<FixHit>> = Vec::new();
    // Assigned by the first lint pass before any read (loop body or the
    // post-budget convergence check below).
    let mut diagnostics: Vec<LintDiagnostic>;

    for _ in 0..max_iterations {
        diagnostics = lint(&current).map_err(LintFixError::Lint)?;
        let batch = fix_batch(current.len(), &diagnostics);
        if batch.is_empty() {
            return Ok(FixpointOutcome {
                source: current,
                iterations: rounds.len(),
                rounds,
                diagnostics,
            });
        }
        current = apply_fix_batch(&current, &batch).map_err(LintFixError::Apply)?;
        rounds.push(batch);
    }

    // Budget spent: one final lint pass decides convergence honestly.
    diagnostics = lint(&current).map_err(LintFixError::Lint)?;
    if fix_batch(current.len(), &diagnostics).is_empty() {
        Ok(FixpointOutcome {
            source: current,
            iterations: rounds.len(),
            rounds,
            diagnostics,
        })
    } else {
        Err(LintFixError::NotConverged { max_iterations })
    }
}

impl LintEngine {
    /// Lint `src`, apply the fixable diagnostics, re-lint, and repeat to
    /// a fixpoint (max `max_iterations` rounds) — the engine-side half of
    /// `hnl lint --fix`.
    ///
    /// The returned [`FixpointOutcome::source`] is the fixed text and
    /// [`FixpointOutcome::rounds`] the per-round [`FixHit`]s; handing
    /// each hit's `fix` fields plus `rule` to `hnl-tool-codemod`'s
    /// `ByteEdit` and calling
    /// `CodemodEngine::plan_byte_edits` per artifact turns the result
    /// into journaled, revertable `Change`s (the host-side wiring seam —
    /// see the module docs).
    ///
    /// # Errors
    ///
    /// [`LintFixError::Lint`] if any pass fails to lex/parse or a rule
    /// errors; [`LintFixError::Apply`] on an overlapping/out-of-bounds
    /// batch; [`LintFixError::NotConverged`] past the budget.
    pub fn fix_source<L, P>(
        &self,
        lang: &handl_language::Language,
        src: &str,
        lexer: &L,
        parser: &P,
        max_iterations: usize,
    ) -> Result<FixpointOutcome, LintFixError>
    where
        L: hnl_tool_lex::LexerEngine + ?Sized,
        P: hnl_tool_parse::ParserEngine + ?Sized,
    {
        fixpoint(src, max_iterations, |s| {
            self.lint_source(lang, s, lexer, parser)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiagnosticLevel;

    fn diag(rule: &str, start: usize, end: usize, replacement: &str) -> LintDiagnostic {
        LintDiagnostic {
            level: DiagnosticLevel::Warning,
            message: format!("{rule} fired"),
            span: None,
            rule: rule.to_string(),
            fix: Some(LintFixOut {
                start,
                end,
                replacement: replacement.to_string(),
            }),
        }
    }

    fn no_fix_diag(rule: &str) -> LintDiagnostic {
        LintDiagnostic {
            level: DiagnosticLevel::Info,
            message: format!("{rule} fired"),
            span: None,
            rule: rule.to_string(),
            fix: None,
        }
    }

    #[test]
    fn fix_batch_collects_fixes_with_rule_provenance() {
        let diags = vec![
            diag("rule-a", 0, 2, "x"),
            no_fix_diag("rule-b"),
            diag("rule-c", 4, 6, "y"),
        ];
        let batch = fix_batch(10, &diags);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].rule, "rule-a");
        assert_eq!(batch[1].rule, "rule-c");
    }

    #[test]
    fn whole_file_fix_subsumes_all_others() {
        // The `source.fmt` precedent: a 0..len fix overlaps everything.
        let diags = vec![diag("fmt", 0, 10, "formatted"), diag("rule-a", 2, 4, "x")];
        let batch = fix_batch(10, &diags);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].rule, "fmt");
        assert_eq!(batch[0].fix.replacement, "formatted");
    }

    #[test]
    fn whole_file_fix_on_empty_source_is_not_privileged() {
        // src_len == 0: a 0..0 "fix" is degenerate, not a whole-file
        // rewrite — the batch policy does not fire.
        let diags = vec![diag("r", 0, 0, "x")];
        let batch = fix_batch(0, &diags);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn apply_fix_batch_splices_descending() {
        let hits = fix_batch(11, &[diag("a", 0, 1, "A"), diag("b", 6, 11, "Z")]);
        assert_eq!(apply_fix_batch("hello world", &hits).unwrap(), "Aello Z");
    }

    #[test]
    fn apply_fix_batch_rejects_overlap() {
        let hits = fix_batch(10, &[diag("a", 0, 5, "x"), diag("b", 4, 8, "y")]);
        assert!(matches!(
            apply_fix_batch("0123456789", &hits),
            Err(FixError::Overlapping { .. })
        ));
    }

    #[test]
    fn fixpoint_converges_after_two_rounds() {
        // Fake lint: round 1 fixes "aa"→"b"; round 2 fixes "b c"→"bc";
        // round 3 clean. Modeled as a stateful closure over call count.
        let calls = std::cell::Cell::new(0);
        let out = fixpoint("aa c", 5, |_s| {
            calls.set(calls.get() + 1);
            match calls.get() {
                1 => Ok(vec![diag("r1", 0, 2, "b")]),
                2 => Ok(vec![diag("r2", 1, 2, "")]),
                _ => Ok(vec![no_fix_diag("clean")]),
            }
        })
        .unwrap();
        assert_eq!(out.source, "bc");
        assert_eq!(out.iterations, 2);
        assert_eq!(out.rounds.len(), 2);
        assert_eq!(out.rounds[0][0].rule, "r1");
        assert_eq!(out.rounds[1][0].rule, "r2");
        // Final pass diagnostics are the clean ones.
        assert!(out.diagnostics.iter().all(|d| d.fix.is_none()));
    }

    #[test]
    fn fixpoint_is_idempotent_on_clean_source() {
        let out = fixpoint("clean", 5, |_| Ok(vec![no_fix_diag("ok")])).unwrap();
        assert_eq!(out.source, "clean");
        assert_eq!(out.iterations, 0);
        assert!(out.rounds.is_empty());
    }

    #[test]
    fn fixpoint_reports_not_converged() {
        // A lint that always emits a fix → budget exhausted.
        let err = fixpoint("x", 2, |_| Ok(vec![diag("r", 0, 1, "y")])).unwrap_err();
        assert!(matches!(
            err,
            LintFixError::NotConverged { max_iterations: 2 }
        ));
    }

    #[test]
    fn fixpoint_propagates_lint_errors() {
        let err = fixpoint("x", 5, |_| Err(LangError::engine("boom".to_string()))).unwrap_err();
        assert!(matches!(err, LintFixError::Lint(_)));
    }
}
