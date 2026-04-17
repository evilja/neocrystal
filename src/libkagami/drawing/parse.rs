use std::convert::Infallible;
#[derive(Debug)]
pub enum DrawingCommand {
    Move(f32,f32),
    MoveN(f32,f32),
    Line(f32,f32),
    CubicBezier(f32,f32,f32,f32,f32,f32),
    CubicBSpline(f32,f32,f32,f32,f32,f32),
    ExtendBSpline(f32,f32),
    CloseBSpline,
    Invalid,
}

impl DrawingCommand {
    pub fn req_f32(e: usize) -> usize {
        match e {
            0 => 2,
            1 => 2,
            2 => 2,
            3 => 6,
            4 => 6,
            5 => 2,
            6 => 0,
            _ => 0,
        }
    }
    pub fn drawmode(s: &str) -> usize {
        match s {
            "m" => 0,
            "n" => 1,
            "l" => 2,
            "b" => 3,
            "s" => 4,
            "p" => 5,
            "c" => 6,
            _ => 999,
        }
    }
    pub fn build_command(e: usize, v: &Vec<f32>) -> Self {
        match e {
            0 => Self::Move(v[0], v[1]),
            1 => Self::MoveN(v[0], v[1]),
            2 => Self::Line(v[0], v[1]),
            3 => Self::CubicBezier(v[0], v[1], v[2], v[3], v[4], v[5]),
            4 => Self::CubicBSpline(v[0], v[1], v[2], v[3], v[4], v[5]),
            5 => Self::ExtendBSpline(v[0], v[1]),
            6 => Self::CloseBSpline,
            _ => Self::Invalid,
        }
    }
}

pub struct Drawing {
    pub commands: Vec<DrawingCommand>
}

impl std::str::FromStr for Drawing {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut out = Self { commands: vec![] };
        let mut mode = 999;
        let mut cvtgv: Vec<f32> = vec![];
        for i in s.trim().split(" ") {
            let c_m = DrawingCommand::drawmode(i);
            if c_m == 999 {
                if mode == 999 {
                    return Ok(out);
                }

                match i.parse::<f32>() {
                    Ok(j) => {
                        cvtgv.push(j);

                        let req = DrawingCommand::req_f32(mode);
                        if cvtgv.len() == req {
                            out.commands.push(DrawingCommand::build_command(mode, &cvtgv));
                            cvtgv.clear();
                        }
                    }
                    Err(_) => return Ok(out),
                }
            } else {
                mode = c_m;
                if DrawingCommand::req_f32(mode) == 0 {
                    out.commands.push(DrawingCommand::build_command(mode, &vec![]));
                }
            }
            println!("i: {} mode: {} cvtgv: {:?}", i, mode, cvtgv);
        }
        return Ok(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_move() {
        let d = Drawing::from_str("m 100 200").unwrap();
        assert!(matches!(d.commands[0], DrawingCommand::Move(100.0, 200.0)));
    }

    #[test]
    fn test_move_n() {
        let d = Drawing::from_str("n 100 200").unwrap();
        assert!(matches!(d.commands[0], DrawingCommand::MoveN(100.0, 200.0)));
    }

    #[test]
    fn test_line_single() {
        let d = Drawing::from_str("m 0 0 l 100 200").unwrap();
        assert!(matches!(d.commands[1], DrawingCommand::Line(100.0, 200.0)));
    }

    #[test]
    fn test_line_multiple() {
        let d = Drawing::from_str("m 0 0 l 100 200 300 400 500 600").unwrap();
        assert_eq!(d.commands.len(), 4);
        assert!(matches!(d.commands[1], DrawingCommand::Line(100.0, 200.0)));
        assert!(matches!(d.commands[2], DrawingCommand::Line(300.0, 400.0)));
        assert!(matches!(d.commands[3], DrawingCommand::Line(500.0, 600.0)));
    }

    #[test]
    fn test_cubic_bezier_single() {
        let d = Drawing::from_str("m 0 0 b 1055 694 665 431 470 300").unwrap();
        assert!(matches!(
            d.commands[1],
            DrawingCommand::CubicBezier(1055.0, 694.0, 665.0, 431.0, 470.0, 300.0)
        ));
    }

    #[test]
    fn test_cubic_bezier_multiple() {
        let d = Drawing::from_str("m 0 0 b 1055 694 665 431 470 300 558 278 869 564 824 212").unwrap();
        assert_eq!(d.commands.len(), 3);
        assert!(matches!(
            d.commands[1],
            DrawingCommand::CubicBezier(1055.0, 694.0, 665.0, 431.0, 470.0, 300.0)
        ));
        assert!(matches!(
            d.commands[2],
            DrawingCommand::CubicBezier(558.0, 278.0, 869.0, 564.0, 824.0, 212.0)
        ));
    }

    #[test]
    fn test_bspline() {
        let d = Drawing::from_str("m 0 0 s 100 200 300 400 500 600").unwrap();
        assert!(matches!(
            d.commands[1],
            DrawingCommand::CubicBSpline(100.0, 200.0, 300.0, 400.0, 500.0, 600.0)
        ));
    }

    #[test]
    fn test_extend_bspline() {
        let d = Drawing::from_str("m 0 0 s 100 200 300 400 500 600 p 700 800").unwrap();
        assert!(matches!(d.commands[2], DrawingCommand::ExtendBSpline(700.0, 800.0)));
    }

    #[test]
    fn test_close_bspline() {
        let d = Drawing::from_str("m 0 0 s 100 200 300 400 500 600 c").unwrap();
        assert!(matches!(d.commands[2], DrawingCommand::CloseBSpline));
    }

    #[test]
    fn test_mode_switch() {
        let d = Drawing::from_str("m 472 708 l 332 406 1224 342 1250 826 b 1055 694 665 431 470 300 558 278 869 564 824 212 985 260 1307 357 1468 406").unwrap();
        assert_eq!(d.commands.len(), 7);
        assert!(matches!(d.commands[0], DrawingCommand::Move(472.0, 708.0)));
        assert!(matches!(d.commands[1], DrawingCommand::Line(332.0, 406.0)));
        assert!(matches!(d.commands[2], DrawingCommand::Line(1224.0, 342.0)));
        assert!(matches!(d.commands[3], DrawingCommand::Line(1250.0, 826.0)));
        assert!(matches!(d.commands[4], DrawingCommand::CubicBezier(1055.0, 694.0, 665.0, 431.0, 470.0, 300.0)));
        assert!(matches!(d.commands[5], DrawingCommand::CubicBezier(558.0, 278.0, 869.0, 564.0, 824.0, 212.0)));
        assert!(matches!(d.commands[6], DrawingCommand::CubicBezier(985.0, 260.0, 1307.0, 357.0, 1468.0, 406.0)));
    }

    #[test]
    fn test_float_coords() {
        let d = Drawing::from_str("m 1.5 2.7 l 3.14 6.28").unwrap();
        assert!(matches!(d.commands[0], DrawingCommand::Move(a, b) if (a - 1.5).abs() < 1e-5 && (b - 2.7).abs() < 1e-5));
        assert!(matches!(d.commands[1], DrawingCommand::Line(a, b) if (a - 3.14).abs() < 1e-5 && (b - 6.28).abs() < 1e-5));
    }

    #[test]
    fn test_empty_string() {
        let d = Drawing::from_str("").unwrap();
        assert_eq!(d.commands.len(), 0);
    }

    #[test]
    fn test_incomplete_coords_ignored() {
        // only 1 coord for a mode that needs 2 — should not emit a command
        let d = Drawing::from_str("m 100").unwrap();
        assert_eq!(d.commands.len(), 0);
    }

    #[test]
    fn test_leading_trailing_whitespace() {
        let d = Drawing::from_str("  m 100 200  ").unwrap();
        assert!(matches!(d.commands[0], DrawingCommand::Move(100.0, 200.0)));
    }

    #[test]
    fn test_invalid_token_stops_parsing() {
        let d = Drawing::from_str("m 100 200 l 300 400 INVALID 999 999").unwrap();
        // should stop at INVALID since it's not a valid f32 or mode
        assert_eq!(d.commands.len(), 2);
    }
}
