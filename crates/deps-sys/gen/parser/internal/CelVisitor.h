
// Generated from external/cel-cpp+/parser/internal/Cel.g4 by ANTLR 4.13.2

#pragma once


#include "antlr4-runtime.h"
#include "CelParser.h"


namespace cel_parser_internal {

/**
 * This class defines an abstract visitor for a parse tree
 * produced by CelParser.
 */
class  CelVisitor : public antlr4::tree::AbstractParseTreeVisitor {
public:

  /**
   * Visit parse trees produced by CelParser.
   */
    virtual std::any visitStart(CelParser::StartContext *context) = 0;

    virtual std::any visitExpr(CelParser::ExprContext *context) = 0;

    virtual std::any visitConditionalOr(CelParser::ConditionalOrContext *context) = 0;

    virtual std::any visitConditionalAnd(CelParser::ConditionalAndContext *context) = 0;

    virtual std::any visitRelation(CelParser::RelationContext *context) = 0;

    virtual std::any visitCalc(CelParser::CalcContext *context) = 0;

    virtual std::any visitMemberExpr(CelParser::MemberExprContext *context) = 0;

    virtual std::any visitLogicalNot(CelParser::LogicalNotContext *context) = 0;

    virtual std::any visitNegate(CelParser::NegateContext *context) = 0;

    virtual std::any visitMemberCall(CelParser::MemberCallContext *context) = 0;

    virtual std::any visitSelect(CelParser::SelectContext *context) = 0;

    virtual std::any visitPrimaryExpr(CelParser::PrimaryExprContext *context) = 0;

    virtual std::any visitIndex(CelParser::IndexContext *context) = 0;

    virtual std::any visitIdent(CelParser::IdentContext *context) = 0;

    virtual std::any visitGlobalCall(CelParser::GlobalCallContext *context) = 0;

    virtual std::any visitNested(CelParser::NestedContext *context) = 0;

    virtual std::any visitCreateList(CelParser::CreateListContext *context) = 0;

    virtual std::any visitCreateMap(CelParser::CreateMapContext *context) = 0;

    virtual std::any visitCreateMessage(CelParser::CreateMessageContext *context) = 0;

    virtual std::any visitConstantLiteral(CelParser::ConstantLiteralContext *context) = 0;

    virtual std::any visitExprList(CelParser::ExprListContext *context) = 0;

    virtual std::any visitListInit(CelParser::ListInitContext *context) = 0;

    virtual std::any visitFieldInitializerList(CelParser::FieldInitializerListContext *context) = 0;

    virtual std::any visitOptField(CelParser::OptFieldContext *context) = 0;

    virtual std::any visitMapInitializerList(CelParser::MapInitializerListContext *context) = 0;

    virtual std::any visitSimpleIdentifier(CelParser::SimpleIdentifierContext *context) = 0;

    virtual std::any visitEscapedIdentifier(CelParser::EscapedIdentifierContext *context) = 0;

    virtual std::any visitOptExpr(CelParser::OptExprContext *context) = 0;

    virtual std::any visitInt(CelParser::IntContext *context) = 0;

    virtual std::any visitUint(CelParser::UintContext *context) = 0;

    virtual std::any visitDouble(CelParser::DoubleContext *context) = 0;

    virtual std::any visitString(CelParser::StringContext *context) = 0;

    virtual std::any visitBytes(CelParser::BytesContext *context) = 0;

    virtual std::any visitBoolTrue(CelParser::BoolTrueContext *context) = 0;

    virtual std::any visitBoolFalse(CelParser::BoolFalseContext *context) = 0;

    virtual std::any visitNull(CelParser::NullContext *context) = 0;


};

}  // namespace cel_parser_internal
