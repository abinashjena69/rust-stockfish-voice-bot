#![allow(unused_assignments)]
#![allow(unused_variables)]
use std::io::{self, Write, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::fs::File;
use std::collections::HashMap;
use shakmaty::{Chess, Position, CastlingMode, Role, Square, Color, MoveList};
use shakmaty::san::San;
use shakmaty::uci::UciMove;

// ----- NON-BLOCKING VOICE OUTPUT -----
fn speak(text: &str, voice_enabled: bool) {
    if !voice_enabled { return; }
    let _ = Command::new("termux-tts-speak").arg(text).spawn(); 
}

fn speak_move(prefix: &str, san: &str, is_check: bool, is_mate: bool, voice_enabled: bool, idea: &str) {
    if !voice_enabled { return; }
    let clean_san = san.replace("+", "").replace("#", "");
    let mut text = String::new();
    
    if clean_san == "O-O" { text.push_str("Castles short"); } 
    else if clean_san == "O-O-O" { text.push_str("Castles long"); } 
    else {
        for c in clean_san.chars() {
            match c {
                'N' => text.push_str("Knight "), 'B' => text.push_str("Bishop "),
                'R' => text.push_str("Rook "), 'Q' => text.push_str("Queen "), 'K' => text.push_str("King "),
                'x' => text.push_str("takes "),
                'a'..='h' => text.push_str(&format!(" {} ", c.to_ascii_uppercase())),
                '1'..='8' => text.push_str(&format!(" {} ", c)),
                _ => text.push(c),
            }
        }
    }
    if is_mate { text.push_str(", checkmate! The game is over."); } 
    else if is_check { text.push_str(", check!"); }
    
    let final_speech = if idea.is_empty() { format!("{} {}", prefix, text) } 
    else { format!("{} {}. {}", prefix, text, idea) };
    speak(&final_speech, voice_enabled);
}

// ----- UI DIALOG VOICE INPUT -----
fn listen_for_move() -> String {
    loop {
        println!("🎤 Microphone dialog opened on screen...");
        let output = Command::new("termux-dialog").args(["speech", "-i", "Speak your move"]).output().expect("Failed to execute termux-dialog");
        let result_str = String::from_utf8_lossy(&output.stdout).to_string();
        let mut extracted_text = String::new();
        if let Some(start) = result_str.find("\"text\": \"") {
            let after_start = &result_str[start + 9..];
            if let Some(end) = after_start.find("\"") { extracted_text = after_start[..end].to_string(); }
        }
        if extracted_text.trim().is_empty() {
            println!("... caught silence or error. Press ENTER again when ready."); return String::new(); 
        }
        return extracted_text;
    }
}

// ----- AGGRESSIVE FUZZY PHONETIC PARSER -----
fn clean_spoken_move(spoken: &str) -> String {
    let mut s = spoken.to_lowercase();
    s = s.replace(".", "").replace(",", "").replace("-", "").replace("'", "");
    s = s.replace("nitaxseefour", "nxc4").replace("96", "nf6");
    s = s.replace("takes on", "x").replace("captures on", "x").replace("takes", "x").replace("captures", "x");
    s = s.replace("kills", "x").replace("eats", "x").replace("cross", "x").replace("tax", "x").replace("into", "x");
    s = s.replace("knight", "n").replace("night", "n").replace("nite", "n").replace("queen", "q").replace("bishop", "b");
    s = s.replace("rook", "r").replace("ruk", "r").replace("king", "k").replace("pawn", ""); 
    s = s.replace(" to ", " ").replace(" on ", " ").replace(" the ", " ");
    s = s.replace(" one", "1").replace(" two", "2").replace(" too", "2").replace(" three", "3").replace(" tree", "3");
    s = s.replace(" four", "4").replace(" for", "4").replace(" five", "5").replace(" six", "6").replace(" seven", "7").replace(" eight", "8");
    s = s.replace("nine", "n").replace("9", "n").replace(" ", ""); 
    s
}

fn get_san_candidates(input: &str) -> Vec<String> {
    let s = input.trim();
    if s.eq_ignore_ascii_case("O-O") || s.eq_ignore_ascii_case("0-0") || s.eq_ignore_ascii_case("oo") { return vec!["O-O".to_string()]; }
    if s.eq_ignore_ascii_case("O-O-O") || s.eq_ignore_ascii_case("0-0-0") || s.eq_ignore_ascii_case("ooo") { return vec!["O-O-O".to_string()]; }
    let mut capitalized = s.to_lowercase();
    if let Some(_) = capitalized.chars().next() {
        let mut chars = capitalized.chars();
        let first = chars.next().unwrap().to_ascii_uppercase();
        capitalized = format!("{}{}", first, chars.as_str());
    }
    vec![s.to_string(), s.to_lowercase(), capitalized]
}

fn rebuild_board(history: &[String]) -> Chess {
    let mut pos = Chess::default();
    for uci_str in history {
        if let Ok(uci) = uci_str.parse::<UciMove>() {
            if let Ok(m) = uci.to_move(&pos) { pos = pos.play(&m).expect("History rebuilt error"); }
        }
    }
    pos
}

fn print_board(pos: &Chess) {
    println!("\n    a b c d e f g h"); println!("  +-----------------+");
    for rank in (0..8).rev() {
        print!("{} | ", rank + 1);
        for file in 0..8 {
            let sq = shakmaty::Square::new(rank * 8 + file);
            match pos.board().piece_at(sq) {
                Some(piece) => {
                    let mut c = piece.role.char();
                    if piece.color.is_white() { c = c.to_ascii_uppercase(); }
                    print!("{} ", c);
                },
                None => print!(". "),
            }
        }
        println!("| {}", rank + 1);
    }
    println!("  +-----------------+"); println!("    a b c d e f g h\n");
}

fn save_pgn(san_history: &[String]) {
    if san_history.is_empty() { return; }
    if let Ok(mut file) = File::create("saved_game.pgn") {
        for (i, chunk) in san_history.chunks(2).enumerate() {
            if chunk.len() == 2 { write!(file, "{}. {} {} ", i + 1, chunk[0], chunk[1]).unwrap_or(()); } 
            else { write!(file, "{}. {} ", i + 1, chunk[0]).unwrap_or(()); }
        }
        println!("\n💾 Game saved successfully to 'saved_game.pgn' in this folder!");
    }
}

fn piece_value(role: Role) -> i32 {
    match role { Role::Pawn => 1, Role::Knight | Role::Bishop => 3, Role::Rook => 5, Role::Queen => 9, Role::King => 100 }
}
fn piece_name(role: Role) -> &'static str {
    match role { Role::Pawn => "Pawn", Role::Knight => "Knight", Role::Bishop => "Bishop", Role::Rook => "Rook", Role::Queen => "Queen", Role::King => "King" }
}

fn win_prob(cp: i32) -> f64 {
    let k = 0.00368208;
    50.0 + 50.0 * (2.0 / (1.0 + (-k * cp as f64).exp()) - 1.0)
}

fn get_attackers(pos: &Chess, target: Square, attacker_color: Color) -> Vec<Square> {
    let mut attackers = Vec::new();
    let offsets = [(-2,-1), (-2,1), (-1,-2), (-1,2), (1,-2), (1,2), (2,-1), (2,1)];
    for (df, dr) in offsets {
        let rf = (target.file() as i8) + df; let rr = (target.rank() as i8) + dr;
        if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
            let sq = Square::new((rr * 8 + rf) as u32);
            if let Some(p) = pos.board().piece_at(sq) {
                if p.color == attacker_color && p.role == Role::Knight { attackers.push(sq); }
            }
        }
    }
    let pawn_dir = if attacker_color == Color::Black { -1 } else { 1 };
    for df in [-1, 1] {
        let rf = (target.file() as i8) + df; let rr = (target.rank() as i8) - pawn_dir;
        if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
            let sq = Square::new((rr * 8 + rf) as u32);
            if let Some(p) = pos.board().piece_at(sq) {
                if p.color == attacker_color && p.role == Role::Pawn { attackers.push(sq); }
            }
        }
    }
    for df in -1..=1 {
        for dr in -1..=1 {
            if df == 0 && dr == 0 { continue; }
            let rf = (target.file() as i8) + df; let rr = (target.rank() as i8) + dr;
            if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
                let sq = Square::new((rr * 8 + rf) as u32);
                if let Some(p) = pos.board().piece_at(sq) {
                    if p.color == attacker_color && p.role == Role::King { attackers.push(sq); }
                }
            }
        }
    }
    let rays = [(-1,-1), (-1,1), (1,-1), (1,1), (-1,0), (1,0), (0,-1), (0,1)];
    for (i, (df, dr)) in rays.iter().enumerate() {
        let mut step = 1;
        loop {
            let rf = (target.file() as i8) + df * step; let rr = (target.rank() as i8) + dr * step;
            if rf < 0 || rf >= 8 || rr < 0 || rr >= 8 { break; }
            let sq = Square::new((rr * 8 + rf) as u32);
            if let Some(p) = pos.board().piece_at(sq) {
                if p.color == attacker_color {
                    let is_diag = i < 4;
                    if is_diag && (p.role == Role::Bishop || p.role == Role::Queen) { attackers.push(sq); }
                    if !is_diag && (p.role == Role::Rook || p.role == Role::Queen) { attackers.push(sq); }
                }
                break;
            }
            step += 1;
        }
    }
    attackers
}

fn is_square_attacked(pos: &Chess, target: Square, attacker_color: Color) -> bool {
    !get_attackers(pos, target, attacker_color).is_empty()
}

// ----- 🔍 UPGRADED GRANDMASTER TACTICAL DETECTOR -----
fn detect_tactical_motive(pos_before: &Chess, pos_after: &Chess, m: &shakmaty::Move, is_check: bool, is_opponent: bool) -> String {
    let actor = if is_opponent { "Your opponent" } else { "This move" };
    let target_prefix = if is_opponent { "your" } else { "their" };
    
    if pos_after.is_stalemate() { return format!("{} forces a stalemate to draw the game.", actor); }
    
    let mut primary = String::new(); let mut secondary = Vec::new();
    let my_color = pos_before.turn(); let enemy_color = !my_color;
    let is_endgame = pos_before.fullmoves().get() > 20;

    let mut was_attacked = false;
    if let Some(from_sq) = m.from() { was_attacked = is_square_attacked(&pos_before, from_sq, enemy_color); }
    let is_capture = m.capture().is_some() || m.is_en_passant();

    if pos_before.legal_moves().len() == 1 { primary = "plays the only legal forced move".to_string(); }

    if primary.is_empty() {
        if let shakmaty::Move::Castle { .. } = m {
            let to_file = m.to().file() as i8;
            primary = if to_file >= 6 { "castles short to connect the Rooks".to_string() } else { "castles long to connect the Rooks".to_string() };
        } else if let shakmaty::Move::Normal { promotion: Some(role), .. } = m {
            primary = format!("promotes to a {}", piece_name(*role));
        } else if m.is_en_passant() {
            primary = format!("captures {} Pawn via en passant", target_prefix);
        } else if let Some(captured) = m.capture() {
            let cap_val = piece_value(captured); let my_val = piece_value(m.role());
            if my_val == cap_val && cap_val > 0 { primary = format!("trades a {} for {} {}", piece_name(m.role()), target_prefix, piece_name(captured)); }
            else { primary = format!("captures {} {}", target_prefix, piece_name(captured)); }
        }
    }

    if pos_before.is_check() && !is_check {
        if m.role() == Role::King { if primary.is_empty() { primary = "steps the King out of check".to_string(); } } 
        else {
            if primary.is_empty() { primary = "blocks the check".to_string(); } else { secondary.push("blocks the check".to_string()); }
        }
    } else if was_attacked && !is_capture {
        if m.role() == Role::King { if primary.is_empty() { primary = "steps the King out of danger".to_string(); } } 
        else { if primary.is_empty() { primary = format!("moves the {} out of danger", piece_name(m.role())); } }
    }

    if is_check {
        let checkers = pos_after.checkers();
        if checkers.count() > 1 { secondary.push("delivers a devastating Double Check".to_string()); }
        else if !checkers.contains(m.to()) { secondary.push("unleashes a Discovered Check".to_string()); }
        else { secondary.push("delivers a Check".to_string()); }
    }

    let to_sq = m.to(); let to_file = to_sq.file() as i8; let to_rank = to_sq.rank() as i8;
    let mut attacked_enemies: Vec<(Role, Square)> = Vec::new(); let mut defended_friends: Vec<(Role, Square)> = Vec::new();

    match m.role() {
        Role::Knight => {
            let offsets = [(-2,-1), (-2,1), (-1,-2), (-1,2), (1,-2), (1,2), (2,-1), (2,1)];
            for (df, dr) in offsets {
                let rf = to_file + df; let rr = to_rank + dr;
                if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
                    let sq = Square::new((rr * 8 + rf) as u32);
                    if let Some(p) = pos_after.board().piece_at(sq) {
                        if p.color == enemy_color { attacked_enemies.push((p.role, sq)); }
                        else if p.color == my_color && sq != to_sq { defended_friends.push((p.role, sq)); }
                    }
                }
            }
        },
        Role::Pawn => {
            let dir = if my_color == Color::White { 1 } else { -1 };
            for df in [-1, 1] {
                let rf = to_file + df; let rr = to_rank + dir;
                if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
                    let sq = Square::new((rr * 8 + rf) as u32);
                    if let Some(p) = pos_after.board().piece_at(sq) {
                        if p.color == enemy_color { attacked_enemies.push((p.role, sq)); }
                        else if p.color == my_color { defended_friends.push((p.role, sq)); }
                    }
                }
            }
        },
        Role::King => {
            for df in -1..=1 {
                for dr in -1..=1 {
                    if df == 0 && dr == 0 { continue; }
                    let rf = to_file + df; let rr = to_rank + dr;
                    if rf >= 0 && rf < 8 && rr >= 0 && rr < 8 {
                        let sq = Square::new((rr * 8 + rf) as u32);
                        if let Some(p) = pos_after.board().piece_at(sq) {
                            if p.color == enemy_color { attacked_enemies.push((p.role, sq)); }
                            else if p.color == my_color { defended_friends.push((p.role, sq)); }
                        }
                    }
                }
            }
        },
        Role::Bishop | Role::Rook | Role::Queen => {
            let rays: &[(i8, i8)] = match m.role() {
                Role::Bishop => &[(-1,-1), (-1,1), (1,-1), (1,1)],
                Role::Rook => &[(-1,0), (1,0), (0,-1), (0,1)],
                Role::Queen => &[(-1,-1), (-1,1), (1,-1), (1,1), (-1,0), (1,0), (0,-1), (0,1)],
                _ => &[],
            };
            for (df, dr) in rays {
                let mut step = 1;
                loop {
                    let rf = to_file + df * step; let rr = to_rank + dr * step;
                    if rf < 0 || rf >= 8 || rr < 0 || rr >= 8 { break; }
                    let sq = Square::new((rr * 8 + rf) as u32);
                    if let Some(p) = pos_after.board().piece_at(sq) {
                        if p.color == enemy_color { attacked_enemies.push((p.role, sq)); }
                        else if p.color == my_color && sq != to_sq { defended_friends.push((p.role, sq)); }
                        break; 
                    }
                    step += 1;
                }
            }
        },
    }

    attacked_enemies.retain(|x| x.0 != Role::King);

    if m.role() == Role::King {
        if let Some(target) = attacked_enemies.iter().max_by_key(|x| piece_value(x.0)) {
            let target_role = target.0; let target_sq = target.1;
            if !is_square_attacked(&pos_after, target_sq, enemy_color) {
                secondary.push(format!("attacks {} undefended {}", target_prefix, piece_name(target_role)));
            }
        }
    } else {
        let mut major_threats: Vec<Role> = attacked_enemies.iter().map(|x| x.0).filter(|&r| piece_value(r) >= 3).collect();
        if major_threats.len() >= 2 { 
            major_threats.sort_by_key(|&r| -piece_value(r));
            secondary.push(format!("forks {} {} and {}", target_prefix, piece_name(major_threats[0]), piece_name(major_threats[1]))); 
        } else if let Some(target) = attacked_enemies.iter().max_by_key(|x| piece_value(x.0)) {
            let target_role = target.0; let target_sq = target.1;
            let target_defended = is_square_attacked(&pos_after, target_sq, enemy_color);
            let my_val = piece_value(m.role()); let enemy_val = piece_value(target_role);
            
            // PERFECT THREAT HIERARCHY
            if my_val == enemy_val && enemy_val > 0 { secondary.push(format!("challenges {} {} to a trade", target_prefix, piece_name(target_role))); }
            else if my_val < enemy_val { secondary.push(format!("threatens {} {}", target_prefix, piece_name(target_role))); }
            else if !target_defended { secondary.push(format!("attacks {} undefended {}", target_prefix, piece_name(target_role))); }
            else if enemy_val >= 3 { secondary.push(format!("applies pressure to {} {}", target_prefix, piece_name(target_role))); }
        }
    }

    let mut new_defenses: Vec<Role> = Vec::new();
    for friend in defended_friends {
        let friend_role = friend.0; let friend_sq = friend.1;
        let was_defended_before = is_square_attacked(&pos_before, friend_sq, my_color);
        if !was_defended_before && is_square_attacked(&pos_before, friend_sq, enemy_color) { new_defenses.push(friend_role); }
    }
    if let Some(&highest_friend) = new_defenses.iter().max_by_key(|&&r| piece_value(r)) {
        secondary.push(format!("defends the {}", piece_name(highest_friend)));
    }

    if !is_capture && m.role() != Role::King {
        for rank in 0..8u32 {
            for file in 0..8u32 {
                let sq = Square::new(rank * 8 + file);
                if let Some(p) = pos_after.board().piece_at(sq) {
                    if p.color == enemy_color && piece_value(p.role) >= 3 && p.role != Role::King {
                        let attackers_now = get_attackers(&pos_after, sq, my_color);
                        let attackers_before = get_attackers(&pos_before, sq, my_color);
                        let has_discovered = attackers_now.iter().any(|&a| a != m.to() && !attackers_before.contains(&a));
                        if has_discovered {
                            secondary.push(format!("opens a discovered attack on {} {}", target_prefix, piece_name(p.role)));
                            break;
                        }
                    }
                }
            }
        }
    }

    if primary.is_empty() {
        match m.role() {
            Role::Pawn => {
                if !new_defenses.is_empty() { primary = "advances a Pawn".to_string(); }
                else if to_file >= 3 && to_file <= 4 { primary = "advances a Pawn to control the center".to_string(); } 
                else if (my_color == Color::Black && to_rank <= 3) || (my_color == Color::White && to_rank >= 4) { primary = "pushes a dangerous passed pawn".to_string(); }
                else { primary = "claims space with a Pawn".to_string(); }
            },
            Role::Knight => {
                if to_file == 0 || to_file == 7 { primary = "moves the Knight to the edge".to_string(); }
                else { primary = "moves the Knight to an active square".to_string(); }
            },
            Role::Bishop => {
                if (to_file == 1 || to_file == 6) && (to_rank == 1 || to_rank == 6) { primary = "fianchettos the Bishop".to_string(); }
                else if is_endgame { primary = "shifts the Bishop".to_string(); }
                else { primary = "develops the Bishop to an active diagonal".to_string(); }
            },
            Role::Rook => {
                if to_file >= 3 && to_file <= 4 { primary = "slides the Rook to the central file".to_string(); }
                else { primary = "activates the Rook".to_string(); }
            },
            Role::Queen => { primary = "shifts the Queen to a new square".to_string(); },
            Role::King => { primary = "steps the King to a safer square".to_string(); }
        }
    }

    let mut final_sentence = String::new();
    if !primary.is_empty() {
        final_sentence.push_str(&primary);
        if !secondary.is_empty() {
            final_sentence.push_str(" and "); final_sentence.push_str(&secondary.join(" and "));
        }
    } else if !secondary.is_empty() { final_sentence = secondary.join(" and "); }
    else { final_sentence = "makes a waiting move".to_string(); }

    format!("{} {}.", actor, final_sentence)
}

fn format_eval(eval: i32) -> String {
    if eval > 9000 { format!("M{}", (10000 - eval) / 100) }
    else if eval < -9000 { format!("-M{}", (10000 + eval) / 100) }
    else { format!("{:.2}", eval as f32 / 100.0) }
}

fn evaluate_realtime_move(
    stdin: &mut std::process::ChildStdin, reader: &mut BufReader<std::process::ChildStdout>, 
    move_history: &[String], depth: u32, is_white_move: bool
) -> (i32, String, String, String) {
    let moves_str = move_history.join(" ");
    if moves_str.is_empty() { writeln!(stdin, "position startpos").unwrap(); } 
    else { writeln!(stdin, "position startpos moves {}", moves_str).unwrap(); }
    stdin.flush().unwrap();
    writeln!(stdin, "setoption name MultiPV value 1").unwrap();
    writeln!(stdin, "go depth {}", depth).unwrap();
    stdin.flush().unwrap();

    let mut cp = 0; let mut bestmove_uci = String::new();
    let mut reply_uci = String::new(); let mut follow_uci = String::new();
    loop {
        let mut line = String::new(); reader.read_line(&mut line).unwrap();
        let trimmed = line.trim();
        if trimmed.starts_with("info depth") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(p) = parts.iter().position(|&x| x == "cp") {
                if p + 1 < parts.len() { cp = parts[p + 1].parse().unwrap_or(0); }
            } else if let Some(p) = parts.iter().position(|&x| x == "mate") {
                if p + 1 < parts.len() {
                    let m_val: i32 = parts[p + 1].parse().unwrap_or(0);
                    cp = if m_val > 0 { 10000 - m_val * 100 } else { -10000 - m_val * 100 };
                }
            }
            if let Some(p) = parts.iter().position(|&x| x == "pv") {
                if p + 1 < parts.len() { bestmove_uci = parts[p+1].to_string(); }
                if p + 2 < parts.len() { reply_uci = parts[p+2].to_string(); }
                if p + 3 < parts.len() { follow_uci = parts[p+3].to_string(); }
            }
        }
        if trimmed.starts_with("bestmove") {
            if bestmove_uci.is_empty() { bestmove_uci = trimmed.split_whitespace().nth(1).unwrap_or("").to_string(); }
            break;
        }
    }
    let eval_score = if is_white_move { cp } else { -cp };
    (eval_score, bestmove_uci, reply_uci, follow_uci)
}

fn run_post_game_review(move_history: &[String], san_history: &[String], live_evals: &[(i32, String, String, String)]) {
    if move_history.is_empty() { return; }
    println!("\n========================================");
    println!("          🏆 POST-GAME REVIEW 🏆        ");
    println!("========================================");
    println!("🔍 Analyzing game logs instantly...");

    let mut w_acc_total = 0.0; let mut b_acc_total = 0.0;
    let mut white_cats: HashMap<&str, Vec<String>> = HashMap::new();
    let mut black_cats: HashMap<&str, Vec<String>> = HashMap::new();
    let mut notes: Vec<String> = Vec::new();
    let mut current_pos = Chess::default();

    for (ply, san) in san_history.iter().enumerate() {
        let eval_before = live_evals[ply].0; let eval_after = live_evals[ply + 1].0;
        let best_uci = &live_evals[ply].1; let is_white_turn = ply % 2 == 0;
        
        let wp_before = win_prob(if is_white_turn { eval_before } else { -eval_before });
        let wp_after = win_prob(if is_white_turn { eval_after } else { -eval_after });
        let mut wp_loss = wp_before - wp_after; if wp_loss < 0.0 { wp_loss = 0.0; }

        let move_acc = (100.0 - (wp_loss * 1.5)).clamp(10.0, 100.0);
        if is_white_turn { w_acc_total += move_acc; } else { b_acc_total += move_acc; }

        let mut best_san = String::from("a safer move");
        if let Ok(uci) = best_uci.parse::<UciMove>() {
            if let Ok(m) = uci.to_move(&current_pos) { best_san = San::from_move(&current_pos, &m).to_string(); }
        }
        
        let p_before = if is_white_turn { eval_before } else { -eval_before };
        let p_after = if is_white_turn { eval_after } else { -eval_after };
        let mut loss = p_before - p_after; if loss < 0 { loss = 0; } 
        
        let cat_map = if is_white_turn { &mut white_cats } else { &mut black_cats };
        let color_name = if is_white_turn { "White" } else { "Black" };
        
        let mut m_opt = None; let mut next_m_opt = None; let mut is_forced = false;

        if let Ok(parsed_san) = san.parse::<San>() {
            is_forced = current_pos.legal_moves().len() == 1;
            if let Ok(mv) = parsed_san.to_move(&current_pos) { 
                m_opt = Some(mv.clone()); 
                let next_pos = current_pos.clone().play(&mv).expect("Valid history"); 
                if ply + 1 < san_history.len() {
                    if let Ok(next_parsed_san) = san_history[ply + 1].parse::<San>() {
                        if let Ok(next_mv) = next_parsed_san.to_move(&next_pos) { next_m_opt = Some(next_mv); }
                    }
                }
                current_pos = next_pos;
            }
        }

        let mut cat = "Best";
        if ply < 4 { cat = "Book"; }
        else if san == &best_san { cat = "Best"; }
        else if loss <= 15 { cat = "Excellent"; }
        else if loss <= 40 { cat = "Good"; }
        else if loss <= 90 { cat = "Inaccuracy"; }
        else if loss <= 200 { cat = "Mistake"; }
        else { cat = if p_before > 300 && p_after < 50 { "Miss" } else { "Blunder" }; }
        
        if cat == "Best" || cat == "Excellent" {
            if let Some(m) = m_opt {
                let moved_val = piece_value(m.role()); let cap_val = m.capture().map(|c| piece_value(c)).unwrap_or(0);
                let mut opponent_recaptured = false;
                if let Some(next_m) = next_m_opt { if next_m.to() == m.to() { opponent_recaptured = true; } }
                
                let is_sacrifice = (moved_val > cap_val && opponent_recaptured) || (cap_val == 0 && opponent_recaptured && m.role() != Role::Pawn);
                if is_sacrifice && !is_forced && loss <= 20 && p_before.abs() < 2000 { cat = "Brilliant"; }
            }
        }

        if cat == "Mistake" || cat == "Blunder" || cat == "Miss" {
            let reason = if p_after < -9000 { "It allows a forced checkmate." } else if p_before > 9000 && p_after < 9000 { "You missed a forced checkmate sequence." } else if p_before > 150 && p_after < -50 { "You threw away a winning position." } else if loss > 300 { "It loses significant material." } else { "It gives up too much positional control." };
            notes.push(format!("❌ [{}] {} is a {}!\n   🎯 You should have played: **{}**\n   💡 Why: {}", color_name, san, cat, best_san, reason));
        } else if cat == "Brilliant" {
            notes.push(format!("✨ [{}] {} is Brilliant! A true sacrifice that shatters the opponent's position.", color_name, san));
        }
        cat_map.entry(cat).or_insert(Vec::new()).push(san.clone());
    }

    let w_moves = (move_history.len() + 1) / 2; let b_moves = move_history.len() / 2;
    let w_acc = if w_moves > 0 { (w_acc_total / w_moves as f64).clamp(10.0, 100.0) } else { 100.0 };
    let b_acc = if b_moves > 0 { (b_acc_total / b_moves as f64).clamp(10.0, 100.0) } else { 100.0 };
    let w_elo = (w_acc * 25.0).clamp(200.0, 3000.0) as i32;
    let b_elo = if b_moves > 0 { (b_acc * 25.0).clamp(200.0, 3000.0) as i32 } else { 0 };

    println!("\n[White] Accuracy: {:.1}% | Est. Elo: {}", w_acc, w_elo);
    println!("[Black] Accuracy: {:.1}% | Est. Elo: {}", b_acc, b_elo);
    
    let categories = ["Book", "Brilliant", "Best", "Excellent", "Good", "Inaccuracy", "Mistake", "Miss", "Blunder"];
    println!("\n--- WHITE MOVES ---");
    for cat in categories.iter() {
        let empty = vec![]; let moves = white_cats.get(*cat).unwrap_or(&empty);
        if moves.is_empty() { println!("  * {}: 0", cat); } else { println!("  * {}: {} ({})", cat, moves.len(), moves.join(", ")); }
    }
    println!("\n--- BLACK MOVES ---");
    for cat in categories.iter() {
        let empty = vec![]; let moves = black_cats.get(*cat).unwrap_or(&empty);
        if moves.is_empty() { println!("  * {}: 0", cat); } else { println!("  * {}: {} ({})", cat, moves.len(), moves.join(", ")); }
    }

    if !notes.is_empty() {
        println!("\n========================================");
        println!("               COACH NOTES              ");
        println!("========================================");
        for note in notes { println!("{}\n", note); }
    }
    println!("========================================\n");
}

fn main() {
    let stockfish_path = "stockfish"; 
    println!("========================================");
    println!("   RUST STOCKFISH VOICE-BOT (V45)       ");
    println!("========================================");

    let mut voice_enabled = true;
    let search_depth: u32;
    loop {
        print!("Select Engine Strength (1=Fast, 2=Medium, 3=Hard): "); io::stdout().flush().unwrap();
        let mut diff_input = String::new(); io::stdin().read_line(&mut diff_input).unwrap();
        match diff_input.trim() {
            "1" => { search_depth = 8; println!("> Strength: FAST (Depth 8)\n"); break; }
            "2" => { search_depth = 12; println!("> Strength: MEDIUM (Depth 12)\n"); break; }
            "3" => { search_depth = 16; println!("> Strength: HARD (Depth 16)\n"); break; }
            _ => println!("❌ Invalid choice. Please type 1, 2, or 3."),
        }
    }

    let playing_as_white;
    loop {
        print!("Which color do you want to play? (w / b): "); io::stdout().flush().unwrap();
        let mut color_input = String::new(); io::stdin().read_line(&mut color_input).unwrap();
        let c = color_input.trim().to_lowercase();
        if c == "w" || c == "white" { playing_as_white = true; println!("> You are playing as WHITE.\n"); break; } 
        else if c == "b" || c == "black" { playing_as_white = false; println!("> You are playing as BLACK.\n"); break; } 
        else { println!("❌ Invalid input. Please type 'w' for white or 'b' for black."); }
    }

    let mut child = Command::new(stockfish_path).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("Failed to launch Stockfish");
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout_handle = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout_handle);

    writeln!(&mut stdin, "uci").unwrap(); stdin.flush().unwrap();
    loop { let mut line = String::new(); reader.read_line(&mut line).unwrap(); if line.trim() == "uciok" { break; } }
    writeln!(&mut stdin, "isready").unwrap(); stdin.flush().unwrap();
    loop { let mut line = String::new(); reader.read_line(&mut line).unwrap(); if line.trim() == "readyok" { break; } }

    speak("Engine ready.", voice_enabled);
    println!("Stockfish is ready! Type 'undo' anytime to take back a mistake.");
    println!("🎤 NEW VOICE HOTKEY: Hit SPACEBAR + ENTER (or type 'v') to instantly use voice!");
    println!("(Pressing just ENTER without spacebar will confirm the expected move)\n");

    let mut move_history: Vec<String> = Vec::new();
    let mut san_history: Vec<String> = Vec::new();
    let mut pos = Chess::default(); 
    let mut live_evals_data: Vec<(i32, String, String, String)> = Vec::new();
    
    let initial_eval = evaluate_realtime_move(&mut stdin, &mut reader, &move_history, search_depth, pos.turn().is_white());
    live_evals_data.push(initial_eval);
    
    print_board(&pos);

    'game_loop: loop {
        if pos.is_checkmate() {
            println!("🏆 CHECKMATE! The game is over."); speak("Checkmate. The game is over.", voice_enabled); break 'game_loop;
        } else if pos.is_game_over() {
            println!("🤝 GAME OVER! (Draw, Stalemate, or Insufficient Material)"); speak("Game over. It is a draw.", voice_enabled); break 'game_loop;
        }

        let is_user_turn = (move_history.len() % 2 == 0) == playing_as_white;

        if is_user_turn {
            let (eval_before, best_before_uci, reply_uci, follow_uci) = live_evals_data.last().unwrap().clone();
            
            let mut expected_reply = String::new(); let mut follow_up = String::new();
            let mut san_output = String::new(); let mut recommended_move = None;
            let mut is_check = false; let mut is_mate = false; let mut idea = String::new();
            let eval_score = format_eval(eval_before);

            if let Ok(uci) = best_before_uci.parse::<UciMove>() {
                if let Ok(m) = uci.to_move(&pos) {
                    san_output = San::from_move(&pos, &m).to_string();
                    recommended_move = Some(m.clone());
                    if let Ok(next_pos) = pos.clone().play(&m) {
                        if let Ok(r_uci) = reply_uci.parse::<UciMove>() {
                            if let Ok(rm) = r_uci.to_move(&next_pos) {
                                expected_reply = San::from_move(&next_pos, &rm).to_string();
                                if let Ok(p2) = next_pos.clone().play(&rm) {
                                    if let Ok(f_uci) = follow_uci.parse::<UciMove>() {
                                        if let Ok(fm) = f_uci.to_move(&p2) { follow_up = San::from_move(&p2, &fm).to_string(); }
                                    }
                                }
                            }
                        }
                        is_check = next_pos.is_check(); is_mate = next_pos.is_checkmate();
                        idea = detect_tactical_motive(&pos, &next_pos, &m, is_check, false);
                        if !expected_reply.is_empty() && !follow_up.is_empty() { idea.push_str(&format!(" If they reply with {}, we follow up with {}.", expected_reply, follow_up)); }
                    }
                }
            }

            let check_tag = if is_mate { " (Checkmate!)" } else if is_check { " (Check!)" } else { "" };
            println!("🤖 Stockfish suggests: **{}{}**   [Eval: {}]", san_output, check_tag, eval_score);
            if !idea.is_empty() { println!("💡 Idea: {}", idea); }
            speak_move("I suggest", &san_output, is_check, is_mate, voice_enabled, &idea);

            loop {
                print!("\nYour move (ENTER for '**{}**', Space+ENTER for Voice, or type override): ", san_output);
                io::stdout().flush().unwrap();

                let mut input = String::new(); io::stdin().read_line(&mut input).unwrap();
                let raw_cmd = input.replace("\n", "").replace("\r", "");
                let lower_cmd = raw_cmd.trim().to_lowercase();
                let mut voice_used = false;

                if lower_cmd == "quit" { break 'game_loop; }
                if lower_cmd == "mute" { voice_enabled = false; println!("🔇 Voice Muted.\n"); continue; }
                if lower_cmd == "unmute" { voice_enabled = true; println!("🔊 Voice Unmuted.\n"); continue; }
                if lower_cmd == "undo" {
                    if move_history.len() > 0 {
                        move_history.pop(); san_history.pop(); live_evals_data.pop();
                        pos = rebuild_board(&move_history); print_board(&pos); println!("⏪ Undo successful! Board reversed.\n");
                    }
                    break;
                }

                let mut processed_cmd = lower_cmd.clone();

                if raw_cmd == "" { 
                    if let Some(m) = recommended_move {
                        let uci_str = m.to_uci(CastlingMode::Standard).to_string();
                        let next_pos = pos.clone().play(&m).expect("Stockfish invalid move");
                        pos = next_pos; move_history.push(uci_str); san_history.push(san_output.clone());
                        
                        let next_eval = evaluate_realtime_move(&mut stdin, &mut reader, &move_history, search_depth, pos.turn().is_white());
                        live_evals_data.push(next_eval);
                        
                        print_board(&pos); println!("> Registered move: **{}** [⭐ Best]\n", san_output); break;
                    }
                } else if raw_cmd == " " || raw_cmd == "v" { 
                    let spoken = listen_for_move();
                    if spoken.is_empty() { continue; } 
                    processed_cmd = clean_spoken_move(&spoken);
                    voice_used = true;
                } else {
                    processed_cmd = processed_cmd.replace(" ", ""); 
                }

                if voice_used {
                    println!("> 🗣️ Voice recognized as: '{}'", processed_cmd);
                    let spoken_format = processed_cmd.replace("x", "takes "); speak(&format!("Heard {}", spoken_format), voice_enabled);
                    print!("Press ENTER to confirm, or type the correct move: "); io::stdout().flush().unwrap();
                    let mut confirm_input = String::new(); io::stdin().read_line(&mut confirm_input).unwrap();
                    let confirm_cmd = confirm_input.trim().to_string();
                    if !confirm_cmd.is_empty() { processed_cmd = confirm_cmd.replace(" ", ""); println!("> Voice overridden. Using: {}\n", processed_cmd); }
                }

                let candidates = get_san_candidates(&processed_cmd);
                let mut parsed_move = None;
                for cand in &candidates {
                    if let Ok(san) = cand.parse::<San>() { if let Ok(m) = san.to_move(&pos) { parsed_move = Some(m); break; } }
                }
                if parsed_move.is_none() {
                    if let Ok(uci) = processed_cmd.to_lowercase().parse::<UciMove>() { if let Ok(m) = uci.to_move(&pos) { parsed_move = Some(m); } }
                }

                if let Some(valid_move) = parsed_move {
                    let uci_str = valid_move.to_uci(CastlingMode::Standard).to_string();
                    let actual_san = San::from_move(&pos, &valid_move).to_string();
                    let next_pos = pos.clone().play(&valid_move).expect("Move failed");
                    move_history.push(uci_str.clone());
                    
                    let next_eval = evaluate_realtime_move(&mut stdin, &mut reader, &move_history, search_depth, next_pos.turn().is_white());
                    let eval_after = next_eval.0;
                    let punishment_uci = next_eval.1.clone();
                    live_evals_data.push(next_eval);
                    
                    let mut loss = if pos.turn().is_white() { eval_before - eval_after } else { eval_after - eval_before };
                    if loss < 0 { loss = 0; }
                    let category = if loss <= 15 { "[⭐ Best]" } else if loss <= 40 { "[👍 Good]" } else if loss <= 90 { "[?! Inaccuracy]" } else if loss <= 200 { "[❓ Mistake]" } else { "[❌ Blunder]" };
                    
                    // FIX: Live Game Reason Note Added
                    let mut reason_str = String::new();
                    if loss > 40 {
                        let p_before = if pos.turn().is_white() { eval_before } else { -eval_before };
                        let p_after = if pos.turn().is_white() { eval_after } else { -eval_after };
                        if p_after < -9000 { reason_str = " (Allows forced checkmate)".to_string(); }
                        else if p_before > 9000 && p_after < 9000 { reason_str = " (Missed a forced checkmate)".to_string(); }
                        else if p_before > 150 && p_after < -50 { reason_str = " (Threw away a winning position)".to_string(); }
                        else if loss > 300 { reason_str = " (Loses significant material)".to_string(); }
                        else { reason_str = " (Creates a positional weakness)".to_string(); }
                    }

                    let check_text = if next_pos.is_checkmate() { " (Checkmate!)" } else if next_pos.is_check() { " (Check!)" } else { "" };
                    
                    speak_move("No worries, playing", &actual_san, next_pos.is_check(), next_pos.is_checkmate(), voice_enabled, "");
                    pos = next_pos; san_history.push(actual_san.clone()); print_board(&pos);
                    println!("> Registered your move: **{}{}** {}{}", actual_san, check_text, category, reason_str);

                    if loss > 100 {
                        let mut punish_san = punishment_uci.clone();
                        if let Ok(p_uci) = punishment_uci.parse::<UciMove>() {
                            if let Ok(pm) = p_uci.to_move(&pos) { punish_san = San::from_move(&pos, &pm).to_string(); }
                        }
                        println!("⚠️ WARNING: That move allowed a counter-attack! Opponent can respond with **{}**!\n", punish_san);
                    } else { println!(); }
                    break;
                } else { println!("❌ Invalid override! Try typing move again (e.g. 'Nf3', 'e4')."); }
            }
            
        } else {
            loop {
                let (eval_before, best_before_uci, _, _) = live_evals_data.last().unwrap().clone();
                let mut expected_opp_move = String::new();
                if let Ok(uci) = best_before_uci.parse::<UciMove>() {
                    if let Ok(m) = uci.to_move(&pos) { expected_opp_move = San::from_move(&pos, &m).to_string(); }
                }

                let prompt_text = if !expected_opp_move.is_empty() { format!("(ENTER for '{}', Space+ENTER for Voice)", expected_opp_move) } 
                else { "(Space+ENTER for Voice, or Type move)".to_string() };
                
                print!("Opponent's move {}: ", prompt_text); io::stdout().flush().unwrap();
                let mut input = String::new(); io::stdin().read_line(&mut input).unwrap();
                
                let raw_cmd = input.replace("\n", "").replace("\r", "");
                let lower_cmd = raw_cmd.trim().to_lowercase();
                let mut voice_used = false;

                if lower_cmd == "quit" { break 'game_loop; }
                if lower_cmd == "mute" { voice_enabled = false; println!("🔇 Voice Muted.\n"); continue; }
                if lower_cmd == "unmute" { voice_enabled = true; println!("🔊 Voice Unmuted.\n"); continue; }
                if lower_cmd == "undo" {
                    if move_history.len() > 0 { move_history.pop(); san_history.pop(); live_evals_data.pop(); pos = rebuild_board(&move_history); print_board(&pos); println!("⏪ Undo successful! Board reversed.\n"); }
                    break;
                }

                let mut processed_cmd = lower_cmd.clone();
                
                if raw_cmd == "" { 
                    if !expected_opp_move.is_empty() {
                        processed_cmd = expected_opp_move.clone();
                        println!("> Instantly accepted predicted move: **{}**\n", processed_cmd);
                    } else {
                        let spoken = listen_for_move();
                        if spoken.is_empty() { continue; } 
                        processed_cmd = clean_spoken_move(&spoken); voice_used = true;
                    }
                } else if raw_cmd == " " || raw_cmd == "v" { 
                    let spoken = listen_for_move();
                    if spoken.is_empty() { continue; } 
                    processed_cmd = clean_spoken_move(&spoken); voice_used = true;
                } else { processed_cmd = processed_cmd.replace(" ", ""); }

                if voice_used {
                    println!("> 🗣️ Voice recognized as: '{}'", processed_cmd);
                    let spoken_format = processed_cmd.replace("x", "takes "); speak(&format!("Heard {}", spoken_format), voice_enabled);
                    print!("Press ENTER to confirm, or type the correct move: "); io::stdout().flush().unwrap();
                    let mut confirm_input = String::new(); io::stdin().read_line(&mut confirm_input).unwrap();
                    let confirm_cmd = confirm_input.trim().to_string();
                    if !confirm_cmd.is_empty() { processed_cmd = confirm_cmd.replace(" ", ""); println!("> Voice overridden. Using: {}\n", processed_cmd); }
                }

                let candidates = get_san_candidates(&processed_cmd);
                let mut parsed_move = None;
                for cand in &candidates {
                    if let Ok(san) = cand.parse::<San>() { if let Ok(m) = san.to_move(&pos) { parsed_move = Some(m); break; } }
                }
                if parsed_move.is_none() {
                    if let Ok(uci) = processed_cmd.to_lowercase().parse::<UciMove>() { if let Ok(m) = uci.to_move(&pos) { parsed_move = Some(m); } }
                }

                if let Some(valid_move) = parsed_move {
                    let uci_str = valid_move.to_uci(CastlingMode::Standard).to_string();
                    let actual_san = San::from_move(&pos, &valid_move).to_string();
                    let next_pos = pos.clone().play(&valid_move).expect("Move failed");
                    move_history.push(uci_str.clone());
                    
                    let next_eval = evaluate_realtime_move(&mut stdin, &mut reader, &move_history, search_depth, next_pos.turn().is_white());
                    let eval_after = next_eval.0;
                    let punish_uci = next_eval.1.clone();
                    live_evals_data.push(next_eval);
                    
                    let mut loss = if pos.turn().is_white() { eval_before - eval_after } else { eval_after - eval_before };
                    if loss < 0 { loss = 0; }

                    let category = if loss <= 15 { "[⭐ Best]" } else if loss <= 40 { "[👍 Good]" } else if loss <= 90 { "[?! Inaccuracy]" } else if loss <= 200 { "[❓ Mistake]" } else { "[❌ Blunder]" };
                    
                    // FIX: Live Game Reason Note Added
                    let mut reason_str = String::new();
                    if loss > 40 {
                        let p_before = if pos.turn().is_white() { eval_before } else { -eval_before };
                        let p_after = if pos.turn().is_white() { eval_after } else { -eval_after };
                        if p_after < -9000 { reason_str = " (Allows forced checkmate)".to_string(); }
                        else if p_before > 9000 && p_after < 9000 { reason_str = " (Missed a forced checkmate)".to_string(); }
                        else if p_before > 150 && p_after < -50 { reason_str = " (Threw away a winning position)".to_string(); }
                        else if loss > 300 { reason_str = " (Loses significant material)".to_string(); }
                        else { reason_str = " (Creates a positional weakness)".to_string(); }
                    }

                    let is_check = next_pos.is_check(); let is_mate = next_pos.is_checkmate();
                    let check_text = if is_mate { " (Checkmate!)" } else if is_check { " (Check!)" } else { "" };
                    
                    let idea = detect_tactical_motive(&pos, &next_pos, &valid_move, is_check, true);
                    speak_move("Opponent plays", &actual_san, is_check, is_mate, voice_enabled, &idea);
                    pos = next_pos; san_history.push(actual_san.clone()); print_board(&pos);
                    println!("> Processed Move: **{}{}** {}{}", actual_san, check_text, category, reason_str);
                    println!("💡 Idea: {}", idea);

                    if loss > 100 {
                        let mut punish_san = punish_uci.clone();
                        if let Ok(p_uci) = punish_uci.parse::<UciMove>() {
                            if let Ok(pm) = p_uci.to_move(&pos) { punish_san = San::from_move(&pos, &pm).to_string(); }
                        }
                        println!("⚡ PUNISHMENT: Your opponent made an error! Punish them immediately with **{}**!\n", punish_san);
                    } else { println!(); }
                    break;
                } else {
                    println!("❌ Invalid move! (Heard/Typed: '{}'). Double-check color turn or move syntax.\n", processed_cmd); speak("Invalid move", voice_enabled);
                }
            }
        }
    }
    
    save_pgn(&san_history);
    print!("\nGenerate Post-Game Analysis Review? (y/n): "); io::stdout().flush().unwrap();
    let mut review_input = String::new(); io::stdin().read_line(&mut review_input).unwrap();
if review_input.trim().eq_ignore_ascii_case("y") { run_post_game_review(&move_history, &san_history, &live_evals_data); }
}
