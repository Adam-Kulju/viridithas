// The granularity of evaluation in this engine is in centipawns.

pub mod parameters;
pub mod score;

use parameters::Parameters;
use score::S;

use crate::{
    board::Board,
    definitions::{
        BB, BISHOP, BLACK, BN, BP, BQ, BR, KNIGHT, MAX_DEPTH, QUEEN, ROOK, WB, WHITE, WN, WP, WQ,
        WR, KING,
    },
    lookups::{file, init_eval_masks, init_passed_isolated_bb, rank},
};

use super::movegen::{bitboards::{attacks, north_one, south_one}, BitLoop, BB_NONE};

pub const PAWN_VALUE: S = S(87, 155);
pub const KNIGHT_VALUE: S = S(360, 343);
pub const BISHOP_VALUE: S = S(327, 336);
pub const ROOK_VALUE: S = S(488, 546);
pub const QUEEN_VALUE: S = S(1087, 992);

/// The value of checkmate.
/// To recover depth-to-mate, we subtract depth (ply) from this value.
/// e.g. if white has a mate in two ply, the output from a depth-5 search will be
/// `3_000_000 - 2 = 2_999_998`.
pub const MATE_SCORE: i32 = 3_000_000;

/// A threshold over which scores must be mate.
#[allow(clippy::cast_possible_truncation)]
pub const IS_MATE_SCORE: i32 = MATE_SCORE - MAX_DEPTH.ply_to_horizon() as i32;

/// The value of a draw.
pub const DRAW_SCORE: i32 = 0;

#[rustfmt::skip]
pub static PIECE_VALUES: [S; 13] = [
    S(0, 0),
    PAWN_VALUE, KNIGHT_VALUE, BISHOP_VALUE, ROOK_VALUE, QUEEN_VALUE, S(0, 0),
    PAWN_VALUE, KNIGHT_VALUE, BISHOP_VALUE, ROOK_VALUE, QUEEN_VALUE, S(0, 0),
];

/// The malus applied when a pawn has no pawns of its own colour to the left or right.
pub const ISOLATED_PAWN_MALUS: S = S(19, 24);

/// The malus applied when two (or more) pawns of a colour are on the same file.
pub const DOUBLED_PAWN_MALUS: S = S(32, 23);

/// The bonus granted for having two bishops.
pub const BISHOP_PAIR_BONUS: S = S(60, 109);

/// The bonus for having a rook on an open file.
pub const ROOK_OPEN_FILE_BONUS: S = S(53, 0);
/// The bonus for having a rook on a semi-open file.
pub const ROOK_HALF_OPEN_FILE_BONUS: S = S(29, 0);
/// The bonus for having a queen on an open file.
pub const QUEEN_OPEN_FILE_BONUS: S = S(0, 0);
/// The bonus for having a queen on a semi-open file.
pub const QUEEN_HALF_OPEN_FILE_BONUS: S = S(10, 0);

// nonlinear mobility eval tables.
#[rustfmt::skip]
const KNIGHT_MOBILITY_BONUS: [S; 9] = [S(-138, -155), S(-62, -8), S(-18, -18), S(-12, 46), S(-1, 65), S(-3, 85), S(13, 92), S(32, 91), S(45, 79)];
#[rustfmt::skip]
const BISHOP_MOBILITY_BONUS: [S; 14] = [S(-24, -135), S(-45, 17), S(-1, -39), S(4, 27), S(21, 41), S(34, 54), S(45, 67), S(55, 78), S(61, 84), S(69, 79), S(76, 82), S(100, 51), S(122, 85), S(97, 43)];
#[rustfmt::skip]
const ROOK_MOBILITY_BONUS: [S; 15] = [S(-132, -156), S(-73, -52), S(-7, 9), S(-8, 55), S(-5, 118), S(1, 116), S(6, 136), S(13, 145), S(16, 151), S(25, 157), S(25, 169), S(39, 173), S(44, 172), S(46, 170), S(14, 197)];
#[rustfmt::skip]
const QUEEN_MOBILITY_BONUS: [S; 28] = [S(-29, -49), S(-16, -29), S(-84, -84), S(42, 8), S(15, -28), S(50, -13), S(51, 47), S(52, 26), S(49, 81), S(51, 140), S(57, 137), S(69, 140), S(77, 150), S(84, 154), S(87, 169), S(89, 184), S(92, 190), S(91, 181), S(92, 186), S(109, 156), S(112, 160), S(132, 137), S(132, 139), S(143, 120), S(168, 112), S(84, 112), S(92, 117), S(53, 145)];

/// The bonus applied when a pawn has no pawns of the opposite colour ahead of it, or to the left or right, scaled by the rank that the pawn is on.
pub static PASSED_PAWN_BONUS: [S; 6] = [
    S(-17, -1), S(-23, 21), S(-15, 51), S(14, 82), S(81, 110), S(138, 169)
];

const PAWN_PHASE: i32 = 1;
const KNIGHT_PHASE: i32 = 10;
const BISHOP_PHASE: i32 = 10;
const ROOK_PHASE: i32 = 20;
const QUEEN_PHASE: i32 = 40;
const TOTAL_PHASE: i32 =
    16 * PAWN_PHASE + 4 * KNIGHT_PHASE + 4 * BISHOP_PHASE + 4 * ROOK_PHASE + 2 * QUEEN_PHASE;

const KING_DANGER_COEFFS: [i32; 3] = [34, 179, -748];

#[allow(dead_code)]
pub static RANK_BB: [u64; 8] = init_eval_masks().0;
pub static FILE_BB: [u64; 8] = init_eval_masks().1;

pub static WHITE_PASSED_BB: [u64; 64] = init_passed_isolated_bb().0;
pub static BLACK_PASSED_BB: [u64; 64] = init_passed_isolated_bb().1;

pub static ISOLATED_BB: [u64; 64] = init_passed_isolated_bb().2;

/// `game_phase` computes a number between 0 and 256, which is the phase of the game.
/// 0 is the opening, 256 is the endgame.
#[allow(clippy::many_single_char_names)]
pub const fn game_phase(p: u8, n: u8, b: u8, r: u8, q: u8) -> i32 {
    let mut phase = TOTAL_PHASE;
    phase -= PAWN_PHASE * p as i32;
    phase -= KNIGHT_PHASE * n as i32;
    phase -= BISHOP_PHASE * b as i32;
    phase -= ROOK_PHASE * r as i32;
    phase -= QUEEN_PHASE * q as i32;
    phase
}

/// `lerp` linearly interpolates between `a` and `b` by `t`.
/// `t` is between 0 and 256.
pub fn lerp(mg: i32, eg: i32, t: i32) -> i32 {
    // debug_assert!((0..=256).contains(&t));
    let t = t.min(256);
    mg * (256 - t) / 256 + eg * t / 256
}

pub const fn is_mate_score(score: i32) -> bool {
    score.abs() >= IS_MATE_SCORE
}

impl Board {
    pub fn set_eval_params(&mut self, params: Parameters) {
        self.eval_params = params;
    }

    /// Computes a score for the position, from the point of view of the side to move.
    /// This function should strive to be as cheap to call as possible, relying on
    /// incremental updates in make-unmake to avoid recomputation.
    pub fn evaluate(&mut self) -> i32 {
        #![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

        if !self.pieces.any_pawns() && self.is_material_draw() {
            return if self.side == WHITE {
                DRAW_SCORE
            } else {
                -DRAW_SCORE
            };
        }
        let material = self.material[WHITE as usize] - self.material[BLACK as usize];
        let pst = self.pst_vals;

        let pawn_val = self.pawn_structure_term(); // INCREMENTAL UPDATE.
        let bishop_pair_val = self.bishop_pair_term();
        let rook_open_file_val = self.rook_open_file_term();
        let queen_open_file_val = self.queen_open_file_term();
        let (mobility_val, danger_info) = self.mobility();
        let king_danger = self.score_kingdanger(danger_info);

        let mut score = material;
        score += pst;
        score += pawn_val;
        score += bishop_pair_val;
        score += rook_open_file_val;
        score += queen_open_file_val;
        score += mobility_val;
        score += king_danger;

        let score = score.value(self.phase());

        let score = self.preprocess_drawish_scores(score);

        if self.side == WHITE {
            score
        } else {
            -score
        }
    }

    fn unwinnable_for<const SIDE: u8>(&self) -> bool {
        assert!(
            SIDE == WHITE || SIDE == BLACK,
            "unwinnable_for called with invalid side"
        );

        if SIDE == WHITE {
            if self.major_piece_counts[WHITE as usize] != 0 {
                return false;
            }
            if self.minor_piece_counts[WHITE as usize] > 1 {
                return false;
            }
            if self.num(WP) != 0 {
                return false;
            }
            true
        } else {
            if self.major_piece_counts[BLACK as usize] != 0 {
                return false;
            }
            if self.minor_piece_counts[BLACK as usize] > 1 {
                return false;
            }
            if self.num(BP) != 0 {
                return false;
            }
            true
        }
    }

    const fn is_material_draw(&self) -> bool {
        if self.num(WR) == 0 && self.num(BR) == 0 && self.num(WQ) == 0 && self.num(BQ) == 0 {
            if self.num(WB) == 0 && self.num(BB) == 0 {
                if self.num(WN) < 3 && self.num(BN) < 3 {
                    return true;
                }
            } else if (self.num(WN) == 0
                && self.num(BN) == 0
                && self.num(WB).abs_diff(self.num(BB)) < 2)
                || (self.num(WB) + self.num(WN) == 1 && self.num(BB) + self.num(BN) == 1)
            {
                return true;
            }
        } else if self.num(WQ) == 0 && self.num(BQ) == 0 {
            if self.num(WR) == 1 && self.num(BR) == 1 {
                if (self.num(WN) + self.num(WB)) < 2 && (self.num(BN) + self.num(BB)) < 2 {
                    return true;
                }
            } else if self.num(WR) == 1 && self.num(BR) == 0 {
                if (self.num(WN) + self.num(WB)) == 0
                    && ((self.num(BN) + self.num(BB)) == 1 || (self.num(BN) + self.num(BB)) == 2)
                {
                    return true;
                }
            } else if self.num(WR) == 0
                && self.num(BR) == 1
                && (self.num(BN) + self.num(BB)) == 0
                && ((self.num(WN) + self.num(WB)) == 1 || (self.num(WN) + self.num(WB)) == 2)
            {
                return true;
            }
        }
        false
    }

    fn preprocess_drawish_scores(&mut self, score: i32) -> i32 {
        // if we can't win with our material, we clamp the eval to zero.
        if score > 0 && self.unwinnable_for::<{ WHITE }>()
            || score < 0 && self.unwinnable_for::<{ BLACK }>()
        {
            0
        } else {
            score
        }
    }

    pub const fn zugzwang_unlikely(&self) -> bool {
        const ENDGAME_PHASE: i32 = game_phase(3, 0, 0, 2, 0);
        self.big_piece_counts[self.side as usize] > 0 && self.phase() < ENDGAME_PHASE
    }

    fn bishop_pair_term(&self) -> S {
        let w_count = self.num(WB);
        let b_count = self.num(BB);
        if w_count == b_count {
            return S(0, 0);
        }
        if w_count >= 2 {
            return self.eval_params.bishop_pair_bonus;
        }
        if b_count >= 2 {
            return -self.eval_params.bishop_pair_bonus;
        }
        S(0, 0)
    }

    fn pawn_structure_term(&self) -> S {
        /// not a tunable parameter, just how "number of pawns in a file" is mapped to "amount of doubled pawn-ness"
        static DOUBLED_PAWN_MAPPING: [i32; 7] = [0, 0, 1, 2, 3, 4, 5];
        let mut w_score = S(0, 0);
        let (white_pawns, black_pawns) =
            (self.pieces.pawns::<true>(), self.pieces.pawns::<false>());
        for &white_pawn_loc in self.piece_lists[WP as usize].iter() {
            if ISOLATED_BB[white_pawn_loc as usize] & white_pawns == 0 {
                w_score -= self.eval_params.isolated_pawn_malus;
            }

            if WHITE_PASSED_BB[white_pawn_loc as usize] & black_pawns == 0 {
                let rank = rank(white_pawn_loc) as usize;
                w_score += self.eval_params.passed_pawn_bonus[rank - 1];
            }
        }

        let mut b_score = S(0, 0);
        for &black_pawn_loc in self.piece_lists[BP as usize].iter() {
            if ISOLATED_BB[black_pawn_loc as usize] & black_pawns == 0 {
                b_score -= self.eval_params.isolated_pawn_malus;
            }

            if BLACK_PASSED_BB[black_pawn_loc as usize] & white_pawns == 0 {
                let rank = rank(black_pawn_loc) as usize;
                b_score += self.eval_params.passed_pawn_bonus[7 - rank - 1];
            }
        }

        for &file_mask in &FILE_BB {
            let pawns_in_file = (file_mask & white_pawns).count_ones() as usize;
            let multiplier = DOUBLED_PAWN_MAPPING[pawns_in_file];
            w_score -= self.eval_params.doubled_pawn_malus * multiplier;
            let pawns_in_file = (file_mask & black_pawns).count_ones() as usize;
            let multiplier = DOUBLED_PAWN_MAPPING[pawns_in_file];
            b_score -= self.eval_params.doubled_pawn_malus * multiplier;
        }

        w_score - b_score
    }

    fn is_file_open(&self, file: u8) -> bool {
        let mask = FILE_BB[file as usize];
        let pawns = self.pieces.pawns::<true>() | self.pieces.pawns::<false>();
        (mask & pawns) == 0
    }

    fn is_file_halfopen<const SIDE: u8>(&self, file: u8) -> bool {
        let mask = FILE_BB[file as usize];
        let pawns = if SIDE == WHITE {
            self.pieces.pawns::<true>()
        } else {
            self.pieces.pawns::<false>()
        };
        (mask & pawns) == 0
    }

    fn rook_open_file_term(&self) -> S {
        let mut score = S(0, 0);
        for &rook_sq in self.piece_lists[WR as usize].iter() {
            let file = file(rook_sq);
            if self.is_file_open(file) {
                score += self.eval_params.rook_open_file_bonus;
            } else if self.is_file_halfopen::<WHITE>(file) {
                score += self.eval_params.rook_half_open_file_bonus;
            }
        }
        for &rook_sq in self.piece_lists[BR as usize].iter() {
            let file = file(rook_sq);
            if self.is_file_open(file) {
                score -= self.eval_params.rook_open_file_bonus;
            } else if self.is_file_halfopen::<BLACK>(file) {
                score -= self.eval_params.rook_half_open_file_bonus;
            }
        }
        score
    }

    fn queen_open_file_term(&self) -> S {
        let mut score = S(0, 0);
        for &queen_sq in self.piece_lists[WQ as usize].iter() {
            let file = file(queen_sq);
            if self.is_file_open(file) {
                score += self.eval_params.queen_open_file_bonus;
            } else if self.is_file_halfopen::<WHITE>(file) {
                score += self.eval_params.queen_half_open_file_bonus;
            }
        }
        for &queen_sq in self.piece_lists[BQ as usize].iter() {
            let file = file(queen_sq);
            if self.is_file_open(file) {
                score -= self.eval_params.queen_open_file_bonus;
            } else if self.is_file_halfopen::<BLACK>(file) {
                score -= self.eval_params.queen_half_open_file_bonus;
            }
        }
        score
    }

    /// `phase` computes a number between 0 and 256, which is the phase of the game. 0 is the opening, 256 is the endgame.
    pub const fn phase(&self) -> i32 {
        // todo: this can be incrementally updated.
        let pawns = self.num(WP) + self.num(BP);
        let knights = self.num(WN) + self.num(BN);
        let bishops = self.num(WB) + self.num(BB);
        let rooks = self.num(WR) + self.num(BR);
        let queens = self.num(WQ) + self.num(BQ);
        game_phase(pawns, knights, bishops, rooks, queens)
    }

    fn mobility(&mut self) -> (S, KingDangerInfo) {
        let mut king_danger_info = KingDangerInfo {
            attack_units_on_white: 0,
            attack_units_on_black: 0,
        };
        let white_king_area = king_area::<true>(self.king_sq(WHITE));
        let black_king_area = king_area::<false>(self.king_sq(BLACK));
        let mut mob_score = S(0, 0);
        let safe_white_moves = !self.pieces.pawn_attacks::<false>();
        let safe_black_moves = !self.pieces.pawn_attacks::<true>();
        let blockers = self.pieces.occupied();
        for knight_sq in BitLoop::new(self.pieces.knights::<true>()) {
            let attacks = attacks::<KNIGHT>(knight_sq, BB_NONE);
            // extracting kingsafety
            let attacks_on_black_king = attacks & black_king_area;
            let defense_of_white_king = attacks & white_king_area;
            king_danger_info.attack_units_on_black += attacks_on_black_king.count_ones() as i32 * 2;
            king_danger_info.attack_units_on_white -= defense_of_white_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_white_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score += self.eval_params.knight_mobility_bonus[attacks];
        }
        for knight_sq in BitLoop::new(self.pieces.knights::<false>()) {
            let attacks = attacks::<KNIGHT>(knight_sq, BB_NONE);
            // extracting kingsafety
            let attacks_on_white_king = attacks & white_king_area;
            let defense_of_black_king = attacks & black_king_area;
            king_danger_info.attack_units_on_white += attacks_on_white_king.count_ones() as i32 * 2;
            king_danger_info.attack_units_on_black -= defense_of_black_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_black_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score -= self.eval_params.knight_mobility_bonus[attacks];
        }
        for bishop_sq in BitLoop::new(self.pieces.bishops::<true>()) {
            let attacks = attacks::<BISHOP>(bishop_sq, blockers);
            // extracting kingsafety
            let attacks_on_black_king = attacks & black_king_area;
            let defense_of_white_king = attacks & white_king_area;
            king_danger_info.attack_units_on_black += attacks_on_black_king.count_ones() as i32 * 2;
            king_danger_info.attack_units_on_white -= defense_of_white_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_white_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score += self.eval_params.bishop_mobility_bonus[attacks];
        }
        for bishop_sq in BitLoop::new(self.pieces.bishops::<false>()) {
            let attacks = attacks::<BISHOP>(bishop_sq, blockers);
            // extracting kingsafety
            let attacks_on_white_king = attacks & white_king_area;
            let defense_of_black_king = attacks & black_king_area;
            king_danger_info.attack_units_on_white += attacks_on_white_king.count_ones() as i32 * 2;
            king_danger_info.attack_units_on_black -= defense_of_black_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_black_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score -= self.eval_params.bishop_mobility_bonus[attacks];
        }
        for rook_sq in BitLoop::new(self.pieces.rooks::<true>()) {
            let attacks = attacks::<ROOK>(rook_sq, blockers);
            // extracting kingsafety
            let attacks_on_black_king = attacks & black_king_area;
            let defense_of_white_king = attacks & white_king_area;
            king_danger_info.attack_units_on_black += attacks_on_black_king.count_ones() as i32 * 3;
            king_danger_info.attack_units_on_white -= defense_of_white_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_white_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score += self.eval_params.rook_mobility_bonus[attacks];
        }
        for rook_sq in BitLoop::new(self.pieces.rooks::<false>()) {
            let attacks = attacks::<ROOK>(rook_sq, blockers);
            // extracting kingsafety
            let attacks_on_white_king = attacks & white_king_area;
            let defense_of_black_king = attacks & black_king_area;
            king_danger_info.attack_units_on_white += attacks_on_white_king.count_ones() as i32 * 3;
            king_danger_info.attack_units_on_black -= defense_of_black_king.count_ones() as i32;
            // mobility
            let attacks = attacks & safe_black_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score -= self.eval_params.rook_mobility_bonus[attacks];
        }
        for queen_sq in BitLoop::new(self.pieces.queens::<true>()) {
            let attacks = attacks::<QUEEN>(queen_sq, blockers);
            // extracting kingsafety
            let attacks_on_black_king = attacks & black_king_area;
            let defense_of_white_king = attacks & white_king_area;
            king_danger_info.attack_units_on_black += attacks_on_black_king.count_ones() as i32 * 5;
            king_danger_info.attack_units_on_white -= defense_of_white_king.count_ones() as i32 * 2;
            // mobility
            let attacks = attacks & safe_white_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score += self.eval_params.queen_mobility_bonus[attacks];
        }
        for queen_sq in BitLoop::new(self.pieces.queens::<false>()) {
            let attacks = attacks::<QUEEN>(queen_sq, blockers);
            // extracting kingsafety
            let attacks_on_white_king = attacks & white_king_area;
            let defense_of_black_king = attacks & black_king_area;
            king_danger_info.attack_units_on_white += attacks_on_white_king.count_ones() as i32 * 5;
            king_danger_info.attack_units_on_black -= defense_of_black_king.count_ones() as i32 * 2;
            // mobility
            let attacks = attacks & safe_black_moves;
            let attacks = attacks.count_ones() as usize;
            mob_score -= self.eval_params.queen_mobility_bonus[attacks];
        }
        (mob_score, king_danger_info)
    }

    fn score_kingdanger(&self, kd: KingDangerInfo) -> S {
        #![allow(clippy::unused_self)]
        let [a, b, c] = self.eval_params.king_danger_coeffs;
        let kd_formula = |au| { (a * au * au + b * au + c) / 100 };

        let white_attack_strength = kd_formula(kd.attack_units_on_black.clamp(0, 99)).min(500);
        let black_attack_strength = kd_formula(kd.attack_units_on_white.clamp(0, 99)).min(500);
        let relscore = white_attack_strength - black_attack_strength;
        S(relscore, relscore / 2)
    }
}

pub fn king_area<const IS_WHITE: bool>(king_sq: u8) -> u64 {
    let king_attacks = attacks::<KING>(king_sq, BB_NONE);
    let forward_area = if IS_WHITE {
        north_one(king_attacks)
    } else {
        south_one(king_attacks)
    };
    king_attacks | forward_area
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KingDangerInfo {
    attack_units_on_white: i32,
    attack_units_on_black: i32,
}

mod tests {
    #[test]
    fn unwinnable() {
        const FEN: &str = "8/8/8/8/2K2k2/2n2P2/8/8 b - - 1 1";
        crate::magic::initialise();
        let mut board = super::Board::from_fen(FEN).unwrap();
        let eval = board.evaluate();
        assert!(
            eval.abs() == 0,
            "eval is not a draw score ({eval}cp != 0cp) in a position unwinnable for both sides."
        );
    }

    #[test]
    fn turn_equality() {
        const FEN1: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        const FEN2: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        crate::magic::initialise();
        let mut board1 = super::Board::from_fen(FEN1).unwrap();
        let mut board2 = super::Board::from_fen(FEN2).unwrap();
        let eval1 = board1.evaluate();
        let eval2 = board2.evaluate();
        assert_eq!(eval1, -eval2);
    }

    #[test]
    fn startpos_mobility_equality() {
        use crate::board::evaluation::S;
        crate::magic::initialise();
        let mut board = super::Board::default();
        assert_eq!(board.mobility().0, S(0, 0));
    }

    #[test]
    fn startpos_eval_equality() {
        crate::magic::initialise();
        let mut board = super::Board::default();
        assert_eq!(board.evaluate(), 0);
    }

    #[test]
    fn startpos_bits_equality() {
        use crate::board::evaluation::score::S;

        crate::magic::initialise();

        let mut board = super::Board::default();

        let material = board.material[crate::definitions::WHITE as usize] - board.material[crate::definitions::BLACK as usize];
        let pst = board.pst_vals;
        let pawn_val = board.pawn_structure_term();
        let bishop_pair_val = board.bishop_pair_term();
        let mobility_val = board.mobility().0;
        let rook_open_file_val = board.rook_open_file_term();
        let queen_open_file_val = board.queen_open_file_term();

        assert_eq!(material, S(0, 0));
        assert_eq!(pst, S(0, 0));
        assert_eq!(pawn_val, S(0, 0));
        assert_eq!(bishop_pair_val, S(0, 0));
        assert_eq!(mobility_val, S(0, 0));
        assert_eq!(rook_open_file_val, S(0, 0));
        assert_eq!(queen_open_file_val, S(0, 0));
    }

    #[test]
    fn startpos_pawn_structure_equality() {
        use crate::board::evaluation::S;
        crate::magic::initialise();
        let board = super::Board::default();
        assert_eq!(board.pawn_structure_term(), S(0, 0));
    }

    #[test]
    fn startpos_open_file_equality() {
        use crate::board::evaluation::S;
        crate::magic::initialise();
        let board = super::Board::default();
        let rook_points = board.rook_open_file_term();
        let queen_points = board.queen_open_file_term();
        assert_eq!(rook_points + queen_points, S(0, 0));
    }

    #[test]
    fn double_pawn_eval() {
        use super::Board;
        use crate::board::evaluation::DOUBLED_PAWN_MALUS;

        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/5P2/PPPP1PPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let pawn_eval = board.pawn_structure_term();
        assert_eq!(pawn_eval, -DOUBLED_PAWN_MALUS);
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/2P2P2/PPP2PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let pawn_eval = board.pawn_structure_term();
        assert_eq!(pawn_eval, -DOUBLED_PAWN_MALUS * 2);
    }

    #[test]
    fn passers_should_be_pushed() {
        use super::Board;

        let mut starting_rank_passer = Board::from_fen("8/k7/8/8/8/8/K6P/8 w - - 0 1").unwrap();
        let mut end_rank_passer = Board::from_fen("8/k6P/8/8/8/8/K7/8 w - - 0 1").unwrap();

        let starting_rank_eval = starting_rank_passer.evaluate();
        let end_rank_eval = end_rank_passer.evaluate();

        // is should be better to have a passer that is more advanced.
        assert!(
            end_rank_eval > starting_rank_eval,
            "end_rank_eval: {end_rank_eval}, starting_rank_eval: {starting_rank_eval}"
        );
    }
}
