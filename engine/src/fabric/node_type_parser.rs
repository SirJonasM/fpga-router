use crate::{
    Compass, MuxPort, NodeType,
    fabric::node::{LutPort, MuxNode, TilePort},
};
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::{anychar, complete::u8 as parse_u8},
    combinator::{all_consuming, map, value},
    sequence::preceded,
};

fn parse_lut_bel_index(input: &str) -> IResult<&str, char> {
    preceded(tag("L"), anychar).parse(input)
}
pub fn parse_node_type(input: &str) -> IResult<&str, NodeType> {
    all_consuming(alt((
        alt((
            parse_lut_input,
            parse_lut_output,
            parse_lut_set_reset,
            parse_lut_enable,
            parse_lut_carry_in,
            parse_lut_carry_out,
        )),
        alt((parse_carry_in, parse_carry_out, parse_vcc, parse_ground, parse_mux, parse_lut)),
        value(NodeType::Other, tag("Other")),
    )))
    .parse(input)
}
fn parse_lut(input: &str) -> IResult<&str, NodeType> {
    map(anychar, |bel| NodeType::Tile {
        port: TilePort::Lut(bel),
    })
    .parse(input)
}

fn parse_lut_input(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_I"), parse_u8), |(bel_index, _, index)| {
        NodeType::Lut {
            bel: bel_index,
            port: LutPort::Input(index),
        }
    })
    .parse(input)
}

fn parse_lut_output(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_O")), |(bel, _)| NodeType::Lut {
        bel,
        port: LutPort::Output,
    })
    .parse(input)
}

fn parse_lut_carry_in(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_Ci")), |(bel, _)| NodeType::Lut {
        bel,
        port: LutPort::CarryIn,
    })
    .parse(input)
}

fn parse_lut_carry_out(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_Co")), |(bel, _)| NodeType::Lut {
        bel,
        port: LutPort::CarryOut,
    })
    .parse(input)
}

fn parse_lut_enable(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_EN")), |(bel, _)| NodeType::Lut {
        bel,
        port: LutPort::Enable,
    })
    .parse(input)
}

fn parse_lut_set_reset(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_SR")), |(bel, _)| NodeType::Lut {
        bel,
        port: LutPort::SetReset,
    })
    .parse(input)
}

fn parse_carry_in(input: &str) -> IResult<&str, NodeType> {
    map((tag("Ci"), parse_u8), |(_, index)| NodeType::Tile {
        port: TilePort::CarryIn(index),
    })
    .parse(input)
}
fn parse_carry_out(input: &str) -> IResult<&str, NodeType> {
    map((tag("Co"), parse_u8), |(_, index)| NodeType::Tile {
        port: TilePort::CarryOut(index),
    })
    .parse(input)
}

fn parse_vcc(input: &str) -> IResult<&str, NodeType> {
    map((tag("VCC"), parse_u8), |(_, index)| NodeType::Tile {
        port: TilePort::VCC(index),
    })
    .parse(input)
}

fn parse_ground(input: &str) -> IResult<&str, NodeType> {
    map((tag("GND"), parse_u8), |(_, index)| NodeType::Tile {
        port: TilePort::Ground(index),
    })
    .parse(input)
}

fn parse_mux(input: &str) -> IResult<&str, NodeType> {
    alt((parse_jump, parse_direction_double, parse_direction_single)).parse(input)
}

fn parse_wire_point(input: &str) -> IResult<&str, MuxPort> {
    alt((
        map(tag("BEGb"), |_| MuxPort::BeginB),
        map(tag("BEG"), |_| MuxPort::Begin),
        map(tag("MID"), |_| MuxPort::Mid),
        map(tag("END"), |_| MuxPort::End),
    ))
    .parse(input)
}
fn parse_compass_double(input: &str) -> IResult<&str, Compass> {
    alt((
        map(tag("NN"), |_| Compass::North),
        map(tag("EE"), |_| Compass::East),
        map(tag("SS"), |_| Compass::South),
        map(tag("WW"), |_| Compass::West),
    ))
    .parse(input)
}
fn parse_compass(input: &str) -> IResult<&str, Compass> {
    alt((
        map(tag("N"), |_| Compass::North),
        map(tag("E"), |_| Compass::East),
        map(tag("S"), |_| Compass::South),
        map(tag("W"), |_| Compass::West),
    ))
    .parse(input)
}
fn parse_jump(input: &str) -> IResult<&str, NodeType> {
    map(
        (tag("J"), parse_compass, parse_u8, parse_wire_point, parse_u8),
        |(_, direction, length, wire_point, id)| {
            NodeType::Mux(MuxNode {
                direction,
                jump: true,
                double: false,
                length,
                id,
                port: wire_point,
            })
        },
    )
    .parse(input)
}

fn parse_direction_single(input: &str) -> IResult<&str, NodeType> {
    map(
        (parse_compass, parse_u8, parse_wire_point, parse_u8),
        |(direction, length, port, id)| {
            NodeType::Mux(MuxNode {
                direction,
                jump: false,
                double: false,
                length,
                port,
                id,
            })
        },
    )
    .parse(input)
}
fn parse_direction_double(input: &str) -> IResult<&str, NodeType> {
    map(
        (parse_compass_double, parse_u8, parse_wire_point, parse_u8),
        |(direction, length, port, id)| {
            NodeType::Mux(MuxNode {
                direction,
                jump: false,
                double: true,
                length,
                port,
                id,
            })
        },
    )
    .parse(input)
}

#[cfg(test)]
mod test {
    use crate::{
        Compass, MuxPort, NodeType,
        fabric::{
            node::{LutPort, MuxNode, TilePort},
            node_type_parser::parse_node_type,
        },
    };

    #[test]
    fn test_parsing() {
        assert_eq!(
            parse_node_type("A"),
            Ok((
                "",
                NodeType::Tile {
                    port: TilePort::Lut('A')
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_I1"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::Input(1)
                }
            ))
        );
        assert_eq!(
            parse_node_type("LB_I2"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'B',
                    port: LutPort::Input(2)
                }
            ))
        );
        assert_eq!(
            parse_node_type("LB_O"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'B',
                    port: LutPort::Output
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_Ci"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::CarryIn
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_Co"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::CarryOut
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_Co"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::CarryOut
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_EN"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::Enable
                }
            ))
        );
        assert_eq!(
            parse_node_type("LA_SR"),
            Ok((
                "",
                NodeType::Lut {
                    bel: 'A',
                    port: LutPort::SetReset
                }
            ))
        );

        assert_eq!(
            parse_node_type("Ci1"),
            Ok((
                "",
                NodeType::Tile {
                    port: TilePort::CarryIn(1)
                }
            ))
        );
        assert_eq!(
            parse_node_type("Co1"),
            Ok((
                "",
                NodeType::Tile {
                    port: TilePort::CarryOut(1)
                }
            ))
        );
        assert_eq!(
            parse_node_type("GND0"),
            Ok((
                "",
                NodeType::Tile {
                    port: TilePort::Ground(0)
                }
            ))
        );
        assert_eq!(parse_node_type("VCC0"), Ok(("", NodeType::Tile { port: TilePort::VCC(0) })));
        let mut wire = MuxNode {
            direction: Compass::North,
            jump: false,
            double: false,
            length: 1,
            port: MuxPort::BeginB,
            id: 1,
        };
        assert_eq!(parse_node_type("N1BEGb1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.port = MuxPort::Begin;
        assert_eq!(parse_node_type("N1BEG1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.jump = true;
        assert_eq!(parse_node_type("JN1BEG1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.jump = false;
        wire.double = true;
        assert_eq!(parse_node_type("NN1BEG1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.double = false;

        wire.direction = Compass::South;
        assert_eq!(parse_node_type("S1BEG1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.jump = true;
        assert_eq!(parse_node_type("JS1BEG1"), Ok(("", NodeType::Mux(wire.clone()))));
        wire.jump = false;
        wire.double = true;
        assert_eq!(parse_node_type("SS1BEG1"), Ok(("", NodeType::Mux(wire))));
    }
}
