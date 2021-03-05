#![allow(clippy::redundant_closure_call)]

use crate::lexer::{Token, TokenKind as TK};
use crate::{HasLoc, Loc};

use crate::ast;

macro_rules! tok {
    ($p:pat, $loc:ident) => {
        Token {
            value: $p,
            loc: $loc,
        }
    };
    ($p:pat) => {
        Token { value: $p, loc: _ }
    };
}

peg::parser! {
    grammar parser() for [Token] {

        //
        // Statement
        //

        pub rule statement() -> Loc<ast::Statement>
        =
            // expr-lhs := expr;
            lhs:expression_atom() [tok!(TK::Becomes)] rhs:expression() [tok!(TK::SemiColon, end)] {
                Loc::new(
                    lhs.loc().merge(end),
                    ast::Statement::Becomes {
                        lhs,
                        rhs,
                    },
                )
            }

        //
        // Expression
        //

        pub rule expression() -> Loc<ast::Expression> = precedence!{
            x:(@) [tok!(TK::Plus, opl)] y:@ {
                Loc::new(
                    x.loc().merge(y.loc()),
                    ast::Expression::InfixOp { op: ast::InfixOp::Add, args: Box::new([x, y]), },
                )
            }
            x:(@) [tok!(TK::Minus, opl)] y:@ {
                Loc::new(
                    x.loc().merge(y.loc()),
                    ast::Expression::InfixOp { op: ast::InfixOp::Sub, args: Box::new([x, y]), },
                )
            }
            --
            x:(@) [tok!(TK::Star, opl)] y:@ {
                Loc::new(
                    x.loc().merge(y.loc()),
                    ast::Expression::InfixOp { op: ast::InfixOp::Mul, args: Box::new([x, y]), },
                )
            }
            x:(@) [tok!(TK::Slash, opl)] y:@ {
                Loc::new(
                    x.loc().merge(y.loc()),
                    ast::Expression::InfixOp { op: ast::InfixOp::Div, args: Box::new([x, y]), },
                )
            }
            x:(@) [tok!(TK::Mod, opl)] y:@ {
                Loc::new(
                    x.loc().merge(y.loc()),
                    ast::Expression::InfixOp { op: ast::InfixOp::Mod, args: Box::new([x, y]), },
                )
            }
            --
            [tok!(TK::Plus, opl)] arg:@ {
                Loc::new(opl.merge(arg.loc()), ast::Expression::PrefixOp { op: ast::PrefixOp::Plus, expr: Box::new(arg) })
            }
            [tok!(TK::Minus, opl)] arg:@ {
                Loc::new(opl.merge(arg.loc()), ast::Expression::PrefixOp { op: ast::PrefixOp::Minus, expr: Box::new(arg) })
            }
            --
            atom:expression_atom() { atom }
        }

        rule expression_atom() -> Loc<ast::Expression> = precedence!{
            base:@ [tok!(TK::BracketOpen)] idx:expression() [tok!(TK::BracketClose, loc)]
            {
                Loc::new(base.loc().merge(loc), ast::Expression::Index {
                    base: Box::new(base),
                    index: Box::new(idx),
                })
            }
            --
            base:@ [tok!(TK::Dot)] name:identifier()
            [tok!(TK::ParenOpen)] args:arglist() [tok!(TK::ParenClose, loc)]
            {
                Loc::new(base.loc().merge(loc), ast::Expression::DotCall {
                    base: Box::new(base),
                    name,
                    args,
                })
            }
            --
            base:@ [tok!(TK::ParenOpen)] args:arglist() [tok!(TK::ParenClose, loc)] {
                Loc::new(base.loc().merge(loc), ast::Expression::Call {
                    base: Box::new(base),
                    args,
                })
            }
            --
            base:@ [tok!(TK::Dot)] name:identifier() {
                Loc::new(
                    base.loc().merge(name.loc()),
                    ast::Expression::Field { base: Box::new(base), name },
                )
            }
            --
            ident:identifier() {
                Loc::new(ident.loc, ast::Expression::Variable(ident.value))
            }
            l:literal() {
                Loc::new(l.loc, ast::Expression::Literal(l.value))
            }
            [tok!(TK::ParenOpen)] inner:expression() [tok!(TK::ParenClose)] {
                inner
            }
        }

        rule call_arg() -> (Option<Loc<ast::Identifier>>, Loc<ast::Expression>)
        =
            ident:identifier()
            [tok!(TK::Colon)]
            rhs:expression()
            {
                (Some(ident), rhs)
            }
        / e:expression() { (None, e) }

        rule arglist() -> Vec<(Option<Loc<ast::Identifier>>, Loc<ast::Expression>)>
        = args:sep_trailing(<call_arg()>, <[tok!(TK::Comma)]>)


        //
        // Types
        //

        rule type_reference() -> Loc<ast::TypeReference>
        =
            prim:type_primitive() {
                Loc::new(prim.loc, ast::TypeReference::Primitive(prim))
            }
        /   [tok!(TK::Array, al)] [tok!(TK::BracketOpen)]
                [tok!(TK::Integer(size), sl)]
            [tok!(TK::BracketOpen)] [tok!(TK::Of)] ty:type_reference() {
                Loc::new(
                    al.merge(ty.loc),
                    ast::TypeReference::Array {
                        base: Box::new(ty),
                        size: Loc::new(sl, size as usize),
                    },
                )
            }
        /   [tok!(TK::Array, al)] [tok!(TK::Of)] ty:type_reference() {
                Loc::new(
                    al.merge(ty.loc),
                    ast::TypeReference::OpenArray {
                        base: Box::new(ty),
                    },
                )
            }
        /   name:identifier() [tok!(TK::LessThan)]
                gens:sep_trailing(<type_reference()>, <[tok!(TK::Comma)]>)
            [tok!(TK::GreaterThan, end)] {
                Loc::new(
                    name.loc.merge(end),
                    ast::TypeReference::Named {
                        name,
                        generics: gens,
                    }
                )
            }
        /   name:identifier() {
                Loc::new(
                    name.loc,
                    ast::TypeReference::Named {
                        name,
                        generics: vec![],
                    }
                )
            }

        rule type_primitive() -> Loc<ast::PrimitiveType>
        =
            [tok!(TK::TyBool, loc)] { Loc::new(loc, ast::PrimitiveType::Bool) }
        /   [tok!(TK::TyInt, loc)] { Loc::new(loc, ast::PrimitiveType::Int) }
        /   [tok!(TK::TyUInt, loc)] { Loc::new(loc, ast::PrimitiveType::UInt) }
        /   [tok!(TK::TyFloat, loc)] { Loc::new(loc, ast::PrimitiveType::Float) }
        /   [tok!(TK::TyDouble, loc)] { Loc::new(loc, ast::PrimitiveType::Double) }

        /   [tok!(TK::TyBoolVec(n), loc)] { Loc::new(loc, ast::PrimitiveType::BoolVec { components: n }) }

        /   [tok!(TK::TyIntVec(n), loc)] annot:type_prim_vec_annot() {
                Loc::new(loc, ast::PrimitiveType::IntVec {
                    components: n,
                    vtype: annot.0,
                    space: annot.1,
                })
            }
        /   [tok!(TK::TyUIntVec(n), loc)] annot:type_prim_vec_annot() {
                Loc::new(loc, ast::PrimitiveType::UIntVec {
                    components: n,
                    vtype: annot.0,
                    space: annot.1,
                })
            }
        /   [tok!(TK::TyFloatVec(n), loc)] annot:type_prim_vec_annot() {
                Loc::new(loc, ast::PrimitiveType::FloatVec {
                    components: n,
                    vtype: annot.0,
                    space: annot.1,
                })
            }
        /   [tok!(TK::TyDoubleVec(n), loc)] annot:type_prim_vec_annot() {
                Loc::new(loc, ast::PrimitiveType::DoubleVec {
                    components: n,
                    vtype: annot.0,
                    space: annot.1,
                })
            }
        // TODO matrices

        rule type_prim_vec_annot() -> (Option<Loc<ast::VecType>>, Option<Loc<ast::Identifier>>)
        =
            [tok!(TK::Is)] ty:type_vec_type() [tok!(TK::In)] space:identifier() {
                (Some(ty), Some(space))
            }
        /   [tok!(TK::Is)] ty:type_vec_type() {
                (Some(ty), None)
            }
        /   [tok!(TK::In)] space:identifier() {
                (None, Some(space))
            }

        rule type_vec_type() -> Loc<ast::VecType>
        = [tok!(TK::Point, loc)] { Loc::new(loc, ast::VecType::Point) }
        / [tok!(TK::Vector, loc)] { Loc::new(loc, ast::VecType::Vector) }
        / [tok!(TK::Colour, loc)] { Loc::new(loc, ast::VecType::Colour) }

        //
        // Terminals
        //

        rule literal() -> Loc<ast::Literal>
        = [tok!(TK::Integer(x), loc)]
        {
            Loc::new(loc, ast::Literal::Integer(x as i128))
        } /
        [tok!(TK::Float(x), loc)]
        {
            Loc::new(loc, ast::Literal::Float(x))
        }

        rule identifier() -> Loc<ast::Identifier>
        = [tok!(TK::Identifier(i), loc)] { Loc::new(loc, i) }

        //
        // Utils
        //

        rule first<T, S>(x: rule<T>, sep: rule<S>) -> T
        = f:x() sep() { f }

        rule sep_trailing<T, S>(x: rule<T>, sep: rule<S>) -> Vec<T>
        =
            // non trailing (at least one)
            items:first(<x()>, <sep()>)* last:x() { let mut items = items; items.push(last); items }
            // trailing trailing (zero or more)
        /   items:first(<x()>, <sep()>)* { items }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::tokenise;

    fn check_expr_parses(input: &str) -> Loc<ast::Expression> {
        let toks = tokenise(0, input).collect::<Vec<_>>();
        parser::expression(&toks[..]).unwrap()
    }

    #[test]
    fn expr_literal() {
        check_expr_parses("12");
    }

    #[test]
    fn expr_infix() {
        let ast = check_expr_parses("12 + 30");
        let printed = format!("{:?}", ast);

        assert!(printed.contains("InfixOp"));
    }

    #[test]
    fn expr_prefix() {
        let ast = check_expr_parses("12 + -30");
        let printed = format!("{:?}", ast);

        assert!(printed.contains("PrefixOp"));
    }

    #[test]
    fn expr_call() {
        check_expr_parses("f()");
        check_expr_parses("f(1)");
        check_expr_parses("f(1,)");
        check_expr_parses("f(1,2)");
        check_expr_parses("f(1, y: 2)");
        check_expr_parses("f(1,2,)");
    }

    #[test]
    #[should_panic(expected = "ParseError")]
    fn expr_call_only_comma_fails() {
        check_expr_parses("f(,)");
    }

    #[test]
    fn expr_dot_call() {
        check_expr_parses("x.f()");
        check_expr_parses("x.f(1)");
        check_expr_parses("x.f(1,)");
        check_expr_parses("x.f(1,2)");
        check_expr_parses("x.f(1, y: 2)");
        check_expr_parses("x.f(1,2,)");

        let ast = check_expr_parses("x.f(1, 2)");
        let printed = format!("{:?}", ast);
        assert!(printed.contains("DotCall"));
        assert!(!printed.contains("Field"));
    }

    #[test]
    fn expr_field() {
        let ast = check_expr_parses("v.x");
        let printed = format!("{:?}", ast);

        assert!(printed.contains("Field"));
        assert!(!printed.contains("DotCall"));
    }

    #[test]
    fn expr_index() {
        let ast = check_expr_parses("x[1 + 2]");
        let printed = format!("{:?}", ast);

        assert!(printed.contains("Index"));
        assert!(printed.contains("InfixOp"));
    }
}