use crate::{Compass, NodeType, Wire, WirePoint};
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
        alt((
            parse_carry_in,
            parse_carry_out,
            parse_vcc,
            parse_ground,
            parse_direction,
            parse_lut,
        )),
        value(NodeType::Other, tag("Other")),
    )))
    .parse(input)
}
fn parse_lut(input: &str) -> IResult<&str, NodeType> {
    map(anychar, NodeType::Lut).parse(input)
}

fn parse_lut_input(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_I"), parse_u8), |(bel_index, _, index)| {
        NodeType::LutInput(bel_index, index)
    })
    .parse(input)
}

fn parse_lut_output(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_O")), |(bel_index, _)| {
        NodeType::LutOutput(bel_index)
    })
    .parse(input)
}

fn parse_lut_carry_in(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_Ci")), |(bel_index, _)| {
        NodeType::LutCarryIn(bel_index)
    })
    .parse(input)
}

fn parse_lut_carry_out(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_Co")), |(bel_index, _)| {
        NodeType::LutCarryOut(bel_index)
    })
    .parse(input)
}

fn parse_lut_enable(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_EN")), |(bel_index, _)| {
        NodeType::LutEnable(bel_index)
    })
    .parse(input)
}

fn parse_lut_set_reset(input: &str) -> IResult<&str, NodeType> {
    map((parse_lut_bel_index, tag("_SR")), |(bel_index, _)| {
        NodeType::LutSetReset(bel_index)
    })
    .parse(input)
}

fn parse_carry_in(input: &str) -> IResult<&str, NodeType> {
    map((tag("Ci"), parse_u8), |(_, index)| NodeType::CarryIn(index)).parse(input)
}
fn parse_carry_out(input: &str) -> IResult<&str, NodeType> {
    map((tag("Co"), parse_u8), |(_, index)| NodeType::CarryOut(index)).parse(input)
}

fn parse_vcc(input: &str) -> IResult<&str, NodeType> {
    map((tag("VCC"), parse_u8), |(_, index)| NodeType::VCC(index)).parse(input)
}

fn parse_ground(input: &str) -> IResult<&str, NodeType> {
    map((tag("GND"), parse_u8), |(_, index)| NodeType::Ground(index)).parse(input)
}

fn parse_direction(input: &str) -> IResult<&str, NodeType> {
    map(alt((parse_jump, parse_direction_double, parse_direction_single)), |wire| {
        NodeType::Wire(wire)
    })
    .parse(input)
}

fn parse_wire_point(input: &str) -> IResult<&str, WirePoint> {
    alt((
        map(tag("BEGb"), |_| WirePoint::BeginB),
        map(tag("BEG"), |_| WirePoint::Begin),
        map(tag("MID"), |_| WirePoint::Mid),
        map(tag("END"), |_| WirePoint::End),
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
fn parse_jump(input: &str) -> IResult<&str, Wire> {
    map(
        (tag("J"), parse_compass, parse_u8, parse_wire_point, parse_u8),
        |(_, direction, length, wire_point, id)| Wire {
            direction,
            jump: true,
            double: false,
            length,
            wire_point,
            id,
        },
    )
    .parse(input)
}

fn parse_direction_single(input: &str) -> IResult<&str, Wire> {
    map(
        (parse_compass, parse_u8, parse_wire_point, parse_u8),
        |(direction, length, wire_point, id)| Wire {
            direction,
            jump: false,
            double: false,
            length,
            wire_point,
            id,
        },
    )
    .parse(input)
}
fn parse_direction_double(input: &str) -> IResult<&str, Wire> {
    map(
        (parse_compass_double, parse_u8, parse_wire_point, parse_u8),
        |(direction, length, wire_point, id)| Wire {
            direction,
            jump: false,
            double: true,
            length,
            wire_point,
            id,
        },
    )
    .parse(input)
}

#[cfg(test)]
mod test {
    use crate::{Compass, NodeType, Wire, WirePoint, fabric::node_type_parser::parse_node_type};

    #[test]
    fn test_parsing() {
        assert_eq!(parse_node_type("A"), Ok(("", NodeType::Lut('A'))));
        assert_eq!(parse_node_type("LA_I1"), Ok(("", NodeType::LutInput('A', 1))));
        assert_eq!(parse_node_type("LB_I2"), Ok(("", NodeType::LutInput('B', 2))));
        assert_eq!(parse_node_type("LA_O"), Ok(("", NodeType::LutOutput('A'))));
        assert_eq!(parse_node_type("LA_Ci"), Ok(("", NodeType::LutCarryIn('A'))));
        assert_eq!(parse_node_type("LA_Co"), Ok(("", NodeType::LutCarryOut('A'))));
        assert_eq!(parse_node_type("LA_EN"), Ok(("", NodeType::LutEnable('A'))));
        assert_eq!(parse_node_type("LA_SR"), Ok(("", NodeType::LutSetReset('A'))));

        assert_eq!(parse_node_type("Ci1"), Ok(("", NodeType::CarryIn(1))));
        assert_eq!(parse_node_type("Co1"), Ok(("", NodeType::CarryOut(1))));
        assert_eq!(parse_node_type("GND0"), Ok(("", NodeType::Ground(0))));
        assert_eq!(parse_node_type("VCC0"), Ok(("", NodeType::VCC(0))));
        let mut wire = Wire {
            direction: Compass::North,
            jump: false,
            double: false,
            length: 1,
            wire_point: WirePoint::Begin,
            id: 1,
        };
        assert_eq!(parse_node_type("N1BEG1"), Ok(("", NodeType::Wire(wire.clone()))));
        wire.jump = true;
        assert_eq!(parse_node_type("JN1BEG1"), Ok(("", NodeType::Wire(wire.clone()))));
        wire.jump = false;
        wire.double = true;
        assert_eq!(parse_node_type("NN1BEG1"), Ok(("", NodeType::Wire(wire.clone()))));
        wire.double = false;

        wire.direction = Compass::South;
        assert_eq!(parse_node_type("S1BEG1"), Ok(("", NodeType::Wire(wire.clone()))));
        wire.jump = true;
        assert_eq!(parse_node_type("JS1BEG1"), Ok(("", NodeType::Wire(wire.clone()))));
        wire.jump = false;
        wire.double = true;
        assert_eq!(parse_node_type("SS1BEG1"), Ok(("", NodeType::Wire(wire))));
    }
}
