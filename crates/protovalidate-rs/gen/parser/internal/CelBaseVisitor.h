
// Generated from external/cel-cpp+/parser/internal/Cel.g4 by ANTLR 4.13.2

#pragma once


#include "antlr4-runtime.h"
#include "CelVisitor.h"


namespace cel_parser_internal {

/**
 * This class provides an empty implementation of CelVisitor, which can be
 * extended to create a visitor which only needs to handle a subset of the available methods.
 */
class  CelBaseVisitor : public CelVisitor {
public:

  virtual std::any visitStart(CelParser::StartContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitExpr(CelParser::ExprContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitConditionalOr(CelParser::ConditionalOrContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitConditionalAnd(CelParser::ConditionalAndContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitRelation(CelParser::RelationContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitCalc(CelParser::CalcContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitMemberExpr(CelParser::MemberExprContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitLogicalNot(CelParser::LogicalNotContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitNegate(CelParser::NegateContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitMemberCall(CelParser::MemberCallContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitSelect(CelParser::SelectContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitPrimaryExpr(CelParser::PrimaryExprContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitIndex(CelParser::IndexContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitIdent(CelParser::IdentContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitGlobalCall(CelParser::GlobalCallContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitNested(CelParser::NestedContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitCreateList(CelParser::CreateListContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitCreateMap(CelParser::CreateMapContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitCreateMessage(CelParser::CreateMessageContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitConstantLiteral(CelParser::ConstantLiteralContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitExprList(CelParser::ExprListContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitListInit(CelParser::ListInitContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitFieldInitializerList(CelParser::FieldInitializerListContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitOptField(CelParser::OptFieldContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitMapInitializerList(CelParser::MapInitializerListContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitSimpleIdentifier(CelParser::SimpleIdentifierContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitEscapedIdentifier(CelParser::EscapedIdentifierContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitOptExpr(CelParser::OptExprContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitInt(CelParser::IntContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitUint(CelParser::UintContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitDouble(CelParser::DoubleContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitString(CelParser::StringContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitBytes(CelParser::BytesContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitBoolTrue(CelParser::BoolTrueContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitBoolFalse(CelParser::BoolFalseContext *ctx) override {
    return visitChildren(ctx);
  }

  virtual std::any visitNull(CelParser::NullContext *ctx) override {
    return visitChildren(ctx);
  }


};

}  // namespace cel_parser_internal
