use std::ops::RangeInclusive;

use rustc_middle::mir::{self, BasicBlock, CallReturnPlaces, Location, TerminatorEdges};

use super::visitor::ResultsVisitor;
use super::{Analysis, Effect, EffectIndex, Results, SwitchIntTarget};

pub trait Direction {
    const IS_FORWARD: bool;
    const IS_BACKWARD: bool = !Self::IS_FORWARD;

    /// Called by `iterate_to_fixpoint` during initial analysis computation.
    fn apply_effects_in_block<'mir, 'tcx, A>(
        analysis: &mut A,
        body: &mir::Body<'tcx>,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        propagate: impl FnMut(BasicBlock, &A::Domain),
    ) where
        A: Analysis<'tcx>;

    /// Called by `ResultsCursor` to recompute the domain value for a location
    /// in a basic block. Applies all effects between the given `EffectIndex`s.
    ///
    /// `effects.start()` must precede or equal `effects.end()` in this direction.
    fn apply_effects_in_range<'tcx, A>(
        analysis: &mut A,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &mir::BasicBlockData<'tcx>,
        effects: RangeInclusive<EffectIndex>,
    ) where
        A: Analysis<'tcx>;

    /// Called by `ResultsVisitor` to recompute the analysis domain values for
    /// all locations in a basic block (starting from the entry value stored
    /// in `Results`) and to visit them with `vis`.
    fn visit_results_in_block<'mir, 'tcx, A>(
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        results: &mut Results<'tcx, A>,
        vis: &mut impl ResultsVisitor<'mir, 'tcx, A>,
    ) where
        A: Analysis<'tcx>;
}

/// Dataflow that runs from the exit of a block (terminator), to its entry (the first statement).
pub struct Backward;

impl Direction for Backward {
    const IS_FORWARD: bool = false;

    fn apply_effects_in_block<'mir, 'tcx, A>(
        analysis: &mut A,
        body: &mir::Body<'tcx>,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        mut propagate: impl FnMut(BasicBlock, &A::Domain),
    ) where
        A: Analysis<'tcx>,
    {
        let terminator = block_data.terminator();
        let location = Location { block, statement_index: block_data.statements.len() };
        analysis.apply_before_terminator_effect(state, terminator, location);
        analysis.apply_terminator_effect(state, terminator, location);
        for (statement_index, statement) in block_data.statements.iter().enumerate().rev() {
            let location = Location { block, statement_index };
            analysis.apply_before_statement_effect(state, statement, location);
            analysis.apply_statement_effect(state, statement, location);
        }

        let exit_state = state;
        for pred in body.basic_blocks.predecessors()[block].iter().copied() {
            match body[pred].terminator().kind {
                // Apply terminator-specific edge effects.
                //
                // FIXME(ecstaticmorse): Avoid cloning the exit state unconditionally.
                mir::TerminatorKind::Call { destination, target: Some(dest), .. }
                    if dest == block =>
                {
                    let mut tmp = exit_state.clone();
                    analysis.apply_call_return_effect(
                        &mut tmp,
                        pred,
                        CallReturnPlaces::Call(destination),
                    );
                    propagate(pred, &tmp);
                }

                mir::TerminatorKind::InlineAsm { ref targets, ref operands, .. }
                    if targets.contains(&block) =>
                {
                    let mut tmp = exit_state.clone();
                    analysis.apply_call_return_effect(
                        &mut tmp,
                        pred,
                        CallReturnPlaces::InlineAsm(operands),
                    );
                    propagate(pred, &tmp);
                }

                mir::TerminatorKind::Yield { resume, resume_arg, .. } if resume == block => {
                    let mut tmp = exit_state.clone();
                    analysis.apply_call_return_effect(
                        &mut tmp,
                        resume,
                        CallReturnPlaces::Yield(resume_arg),
                    );
                    propagate(pred, &tmp);
                }

                mir::TerminatorKind::SwitchInt { targets: _, ref discr } => {
                    if let Some(mut data) = analysis.get_switch_int_data(block, discr) {
                        let values = &body.basic_blocks.switch_sources()[&(block, pred)];
                        let targets =
                            values.iter().map(|&value| SwitchIntTarget { value, target: block });

                        let mut tmp = None;
                        for target in targets {
                            let tmp = opt_clone_from_or_clone(&mut tmp, exit_state);
                            analysis.apply_switch_int_edge_effect(&mut data, tmp, target);
                            propagate(pred, tmp);
                        }
                    } else {
                        propagate(pred, exit_state)
                    }
                }

                _ => propagate(pred, exit_state),
            }
        }
    }

    fn apply_effects_in_range<'tcx, A>(
        analysis: &mut A,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &mir::BasicBlockData<'tcx>,
        effects: RangeInclusive<EffectIndex>,
    ) where
        A: Analysis<'tcx>,
    {
        let (from, to) = (*effects.start(), *effects.end());
        let terminator_index = block_data.statements.len();

        assert!(from.statement_index <= terminator_index);
        assert!(!to.precedes_in_backward_order(from));

        // Handle the statement (or terminator) at `from`.

        let next_effect = match from.effect {
            // If we need to apply the terminator effect in all or in part, do so now.
            _ if from.statement_index == terminator_index => {
                let location = Location { block, statement_index: from.statement_index };
                let terminator = block_data.terminator();

                if from.effect == Effect::Before {
                    analysis.apply_before_terminator_effect(state, terminator, location);
                    if to == Effect::Before.at_index(terminator_index) {
                        return;
                    }
                }

                analysis.apply_terminator_effect(state, terminator, location);
                if to == Effect::Primary.at_index(terminator_index) {
                    return;
                }

                // If `from.statement_index` is `0`, we will have hit one of the earlier comparisons
                // with `to`.
                from.statement_index - 1
            }

            Effect::Primary => {
                let location = Location { block, statement_index: from.statement_index };
                let statement = &block_data.statements[from.statement_index];

                analysis.apply_statement_effect(state, statement, location);
                if to == Effect::Primary.at_index(from.statement_index) {
                    return;
                }

                from.statement_index - 1
            }

            Effect::Before => from.statement_index,
        };

        // Handle all statements between `first_unapplied_idx` and `to.statement_index`.

        for statement_index in (to.statement_index..next_effect).rev().map(|i| i + 1) {
            let location = Location { block, statement_index };
            let statement = &block_data.statements[statement_index];
            analysis.apply_before_statement_effect(state, statement, location);
            analysis.apply_statement_effect(state, statement, location);
        }

        // Handle the statement at `to`.

        let location = Location { block, statement_index: to.statement_index };
        let statement = &block_data.statements[to.statement_index];
        analysis.apply_before_statement_effect(state, statement, location);

        if to.effect == Effect::Before {
            return;
        }

        analysis.apply_statement_effect(state, statement, location);
    }

    fn visit_results_in_block<'mir, 'tcx, A>(
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        results: &mut Results<'tcx, A>,
        vis: &mut impl ResultsVisitor<'mir, 'tcx, A>,
    ) where
        A: Analysis<'tcx>,
    {
        state.clone_from(results.entry_set_for_block(block));

        vis.visit_block_end(state);

        let loc = Location { block, statement_index: block_data.statements.len() };
        let term = block_data.terminator();
        results.analysis.apply_before_terminator_effect(state, term, loc);
        vis.visit_terminator_before_primary_effect(results, state, term, loc);
        results.analysis.apply_terminator_effect(state, term, loc);
        vis.visit_terminator_after_primary_effect(results, state, term, loc);

        for (statement_index, stmt) in block_data.statements.iter().enumerate().rev() {
            let loc = Location { block, statement_index };
            results.analysis.apply_before_statement_effect(state, stmt, loc);
            vis.visit_statement_before_primary_effect(results, state, stmt, loc);
            results.analysis.apply_statement_effect(state, stmt, loc);
            vis.visit_statement_after_primary_effect(results, state, stmt, loc);
        }

        vis.visit_block_start(state);
    }
}

/// Dataflow that runs from the entry of a block (the first statement), to its exit (terminator).
pub struct Forward;

impl Direction for Forward {
    const IS_FORWARD: bool = true;

    fn apply_effects_in_block<'mir, 'tcx, A>(
        analysis: &mut A,
        _body: &mir::Body<'tcx>,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        mut propagate: impl FnMut(BasicBlock, &A::Domain),
    ) where
        A: Analysis<'tcx>,
    {
        for (statement_index, statement) in block_data.statements.iter().enumerate() {
            let location = Location { block, statement_index };
            analysis.apply_before_statement_effect(state, statement, location);
            analysis.apply_statement_effect(state, statement, location);
        }
        let terminator = block_data.terminator();
        let location = Location { block, statement_index: block_data.statements.len() };
        analysis.apply_before_terminator_effect(state, terminator, location);
        let edges = analysis.apply_terminator_effect(state, terminator, location);

        let exit_state = state;
        match edges {
            TerminatorEdges::None => {}
            TerminatorEdges::Single(target) => propagate(target, exit_state),
            TerminatorEdges::Double(target, unwind) => {
                propagate(target, exit_state);
                propagate(unwind, exit_state);
            }
            TerminatorEdges::AssignOnReturn { return_, cleanup, place } => {
                // This must be done *first*, otherwise the unwind path will see the assignments.
                if let Some(cleanup) = cleanup {
                    propagate(cleanup, exit_state);
                }

                if !return_.is_empty() {
                    analysis.apply_call_return_effect(exit_state, block, place);
                    for &target in return_ {
                        propagate(target, exit_state);
                    }
                }
            }
            TerminatorEdges::SwitchInt { targets, discr } => {
                if let Some(mut data) = analysis.get_switch_int_data(block, discr) {
                    let mut tmp = None;
                    for (value, target) in targets.iter() {
                        let tmp = opt_clone_from_or_clone(&mut tmp, exit_state);
                        analysis.apply_switch_int_edge_effect(&mut data, tmp, SwitchIntTarget {
                            value: Some(value),
                            target,
                        });
                        propagate(target, tmp);
                    }

                    // Once we get to the final, "otherwise" branch, there is no need to preserve
                    // `exit_state`, so pass it directly to `apply_switch_int_edge_effect` to save
                    // a clone of the dataflow state.
                    let otherwise = targets.otherwise();
                    analysis.apply_switch_int_edge_effect(&mut data, exit_state, SwitchIntTarget {
                        value: None,
                        target: otherwise,
                    });
                    propagate(otherwise, exit_state);
                } else {
                    for target in targets.all_targets() {
                        propagate(*target, exit_state);
                    }
                }
            }
        }
    }

    fn apply_effects_in_range<'tcx, A>(
        analysis: &mut A,
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &mir::BasicBlockData<'tcx>,
        effects: RangeInclusive<EffectIndex>,
    ) where
        A: Analysis<'tcx>,
    {
        let (from, to) = (*effects.start(), *effects.end());
        let terminator_index = block_data.statements.len();

        assert!(to.statement_index <= terminator_index);
        assert!(!to.precedes_in_forward_order(from));

        // If we have applied the before affect of the statement or terminator at `from` but not its
        // after effect, do so now and start the loop below from the next statement.

        let first_unapplied_index = match from.effect {
            Effect::Before => from.statement_index,

            Effect::Primary if from.statement_index == terminator_index => {
                debug_assert_eq!(from, to);

                let location = Location { block, statement_index: terminator_index };
                let terminator = block_data.terminator();
                analysis.apply_terminator_effect(state, terminator, location);
                return;
            }

            Effect::Primary => {
                let location = Location { block, statement_index: from.statement_index };
                let statement = &block_data.statements[from.statement_index];
                analysis.apply_statement_effect(state, statement, location);

                // If we only needed to apply the after effect of the statement at `idx`, we are
                // done.
                if from == to {
                    return;
                }

                from.statement_index + 1
            }
        };

        // Handle all statements between `from` and `to` whose effects must be applied in full.

        for statement_index in first_unapplied_index..to.statement_index {
            let location = Location { block, statement_index };
            let statement = &block_data.statements[statement_index];
            analysis.apply_before_statement_effect(state, statement, location);
            analysis.apply_statement_effect(state, statement, location);
        }

        // Handle the statement or terminator at `to`.

        let location = Location { block, statement_index: to.statement_index };
        if to.statement_index == terminator_index {
            let terminator = block_data.terminator();
            analysis.apply_before_terminator_effect(state, terminator, location);

            if to.effect == Effect::Primary {
                analysis.apply_terminator_effect(state, terminator, location);
            }
        } else {
            let statement = &block_data.statements[to.statement_index];
            analysis.apply_before_statement_effect(state, statement, location);

            if to.effect == Effect::Primary {
                analysis.apply_statement_effect(state, statement, location);
            }
        }
    }

    fn visit_results_in_block<'mir, 'tcx, A>(
        state: &mut A::Domain,
        block: BasicBlock,
        block_data: &'mir mir::BasicBlockData<'tcx>,
        results: &mut Results<'tcx, A>,
        vis: &mut impl ResultsVisitor<'mir, 'tcx, A>,
    ) where
        A: Analysis<'tcx>,
    {
        state.clone_from(results.entry_set_for_block(block));

        vis.visit_block_start(state);

        for (statement_index, stmt) in block_data.statements.iter().enumerate() {
            let loc = Location { block, statement_index };
            results.analysis.apply_before_statement_effect(state, stmt, loc);
            vis.visit_statement_before_primary_effect(results, state, stmt, loc);
            results.analysis.apply_statement_effect(state, stmt, loc);
            vis.visit_statement_after_primary_effect(results, state, stmt, loc);
        }

        let loc = Location { block, statement_index: block_data.statements.len() };
        let term = block_data.terminator();
        results.analysis.apply_before_terminator_effect(state, term, loc);
        vis.visit_terminator_before_primary_effect(results, state, term, loc);
        results.analysis.apply_terminator_effect(state, term, loc);
        vis.visit_terminator_after_primary_effect(results, state, term, loc);

        vis.visit_block_end(state);
    }
}

/// An analogue of `Option::get_or_insert_with` that stores a clone of `val` into `opt`, but uses
/// the more efficient `clone_from` if `opt` was `Some`.
///
/// Returns a mutable reference to the new clone that resides in `opt`.
//
// FIXME: Figure out how to express this using `Option::clone_from`, or maybe lift it into the
// standard library?
fn opt_clone_from_or_clone<'a, T: Clone>(opt: &'a mut Option<T>, val: &T) -> &'a mut T {
    if opt.is_some() {
        let ret = opt.as_mut().unwrap();
        ret.clone_from(val);
        ret
    } else {
        *opt = Some(val.clone());
        opt.as_mut().unwrap()
    }
}
