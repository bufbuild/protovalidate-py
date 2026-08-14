
// Generated from external/cel-cpp+/parser/internal/Cel.g4 by ANTLR 4.13.2

#pragma once


#include "antlr4-runtime.h"


namespace cel_parser_internal {


class  CelParser : public antlr4::Parser {
public:
  enum {
    EQUALS = 1, NOT_EQUALS = 2, IN = 3, LESS = 4, LESS_EQUALS = 5, GREATER_EQUALS = 6, 
    GREATER = 7, LOGICAL_AND = 8, LOGICAL_OR = 9, LBRACKET = 10, RPRACKET = 11, 
    LBRACE = 12, RBRACE = 13, LPAREN = 14, RPAREN = 15, DOT = 16, COMMA = 17, 
    MINUS = 18, EXCLAM = 19, QUESTIONMARK = 20, COLON = 21, PLUS = 22, STAR = 23, 
    SLASH = 24, PERCENT = 25, CEL_TRUE = 26, CEL_FALSE = 27, NUL = 28, WHITESPACE = 29, 
    COMMENT = 30, NUM_FLOAT = 31, NUM_INT = 32, NUM_UINT = 33, STRING = 34, 
    BYTES = 35, IDENTIFIER = 36, ESC_IDENTIFIER = 37
  };

  enum {
    RuleStart = 0, RuleExpr = 1, RuleConditionalOr = 2, RuleConditionalAnd = 3, 
    RuleRelation = 4, RuleCalc = 5, RuleUnary = 6, RuleMember = 7, RulePrimary = 8, 
    RuleExprList = 9, RuleListInit = 10, RuleFieldInitializerList = 11, 
    RuleOptField = 12, RuleMapInitializerList = 13, RuleEscapeIdent = 14, 
    RuleOptExpr = 15, RuleLiteral = 16
  };

  explicit CelParser(antlr4::TokenStream *input);

  CelParser(antlr4::TokenStream *input, const antlr4::atn::ParserATNSimulatorOptions &options);

  ~CelParser() override;

  std::string getGrammarFileName() const override;

  const antlr4::atn::ATN& getATN() const override;

  const std::vector<std::string>& getRuleNames() const override;

  const antlr4::dfa::Vocabulary& getVocabulary() const override;

  antlr4::atn::SerializedATNView getSerializedATN() const override;


  class StartContext;
  class ExprContext;
  class ConditionalOrContext;
  class ConditionalAndContext;
  class RelationContext;
  class CalcContext;
  class UnaryContext;
  class MemberContext;
  class PrimaryContext;
  class ExprListContext;
  class ListInitContext;
  class FieldInitializerListContext;
  class OptFieldContext;
  class MapInitializerListContext;
  class EscapeIdentContext;
  class OptExprContext;
  class LiteralContext; 

  class  StartContext : public antlr4::ParserRuleContext {
  public:
    CelParser::ExprContext *e = nullptr;
    StartContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    antlr4::tree::TerminalNode *EOF();
    ExprContext *expr();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  StartContext* start();

  class  ExprContext : public antlr4::ParserRuleContext {
  public:
    CelParser::ConditionalOrContext *e = nullptr;
    antlr4::Token *op = nullptr;
    CelParser::ConditionalOrContext *e1 = nullptr;
    CelParser::ExprContext *e2 = nullptr;
    ExprContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<ConditionalOrContext *> conditionalOr();
    ConditionalOrContext* conditionalOr(size_t i);
    antlr4::tree::TerminalNode *COLON();
    antlr4::tree::TerminalNode *QUESTIONMARK();
    ExprContext *expr();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  ExprContext* expr();

  class  ConditionalOrContext : public antlr4::ParserRuleContext {
  public:
    CelParser::ConditionalAndContext *e = nullptr;
    antlr4::Token *s9 = nullptr;
    std::vector<antlr4::Token *> ops;
    CelParser::ConditionalAndContext *conditionalAndContext = nullptr;
    std::vector<ConditionalAndContext *> e1;
    ConditionalOrContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<ConditionalAndContext *> conditionalAnd();
    ConditionalAndContext* conditionalAnd(size_t i);
    std::vector<antlr4::tree::TerminalNode *> LOGICAL_OR();
    antlr4::tree::TerminalNode* LOGICAL_OR(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  ConditionalOrContext* conditionalOr();

  class  ConditionalAndContext : public antlr4::ParserRuleContext {
  public:
    CelParser::RelationContext *e = nullptr;
    antlr4::Token *s8 = nullptr;
    std::vector<antlr4::Token *> ops;
    CelParser::RelationContext *relationContext = nullptr;
    std::vector<RelationContext *> e1;
    ConditionalAndContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<RelationContext *> relation();
    RelationContext* relation(size_t i);
    std::vector<antlr4::tree::TerminalNode *> LOGICAL_AND();
    antlr4::tree::TerminalNode* LOGICAL_AND(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  ConditionalAndContext* conditionalAnd();

  class  RelationContext : public antlr4::ParserRuleContext {
  public:
    antlr4::Token *op = nullptr;
    RelationContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    CalcContext *calc();
    std::vector<RelationContext *> relation();
    RelationContext* relation(size_t i);
    antlr4::tree::TerminalNode *LESS();
    antlr4::tree::TerminalNode *LESS_EQUALS();
    antlr4::tree::TerminalNode *GREATER_EQUALS();
    antlr4::tree::TerminalNode *GREATER();
    antlr4::tree::TerminalNode *EQUALS();
    antlr4::tree::TerminalNode *NOT_EQUALS();
    antlr4::tree::TerminalNode *IN();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  RelationContext* relation();
  RelationContext* relation(int precedence);
  class  CalcContext : public antlr4::ParserRuleContext {
  public:
    antlr4::Token *op = nullptr;
    CalcContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    UnaryContext *unary();
    std::vector<CalcContext *> calc();
    CalcContext* calc(size_t i);
    antlr4::tree::TerminalNode *STAR();
    antlr4::tree::TerminalNode *SLASH();
    antlr4::tree::TerminalNode *PERCENT();
    antlr4::tree::TerminalNode *PLUS();
    antlr4::tree::TerminalNode *MINUS();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  CalcContext* calc();
  CalcContext* calc(int precedence);
  class  UnaryContext : public antlr4::ParserRuleContext {
  public:
    UnaryContext(antlr4::ParserRuleContext *parent, size_t invokingState);
   
    UnaryContext() = default;
    void copyFrom(UnaryContext *context);
    using antlr4::ParserRuleContext::copyFrom;

    virtual size_t getRuleIndex() const override;

   
  };

  class  LogicalNotContext : public UnaryContext {
  public:
    LogicalNotContext(UnaryContext *ctx);

    antlr4::Token *s19 = nullptr;
    std::vector<antlr4::Token *> ops;
    MemberContext *member();
    std::vector<antlr4::tree::TerminalNode *> EXCLAM();
    antlr4::tree::TerminalNode* EXCLAM(size_t i);

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  MemberExprContext : public UnaryContext {
  public:
    MemberExprContext(UnaryContext *ctx);

    MemberContext *member();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  NegateContext : public UnaryContext {
  public:
    NegateContext(UnaryContext *ctx);

    antlr4::Token *s18 = nullptr;
    std::vector<antlr4::Token *> ops;
    MemberContext *member();
    std::vector<antlr4::tree::TerminalNode *> MINUS();
    antlr4::tree::TerminalNode* MINUS(size_t i);

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  UnaryContext* unary();

  class  MemberContext : public antlr4::ParserRuleContext {
  public:
    MemberContext(antlr4::ParserRuleContext *parent, size_t invokingState);
   
    MemberContext() = default;
    void copyFrom(MemberContext *context);
    using antlr4::ParserRuleContext::copyFrom;

    virtual size_t getRuleIndex() const override;

   
  };

  class  MemberCallContext : public MemberContext {
  public:
    MemberCallContext(MemberContext *ctx);

    antlr4::Token *op = nullptr;
    antlr4::Token *id = nullptr;
    antlr4::Token *open = nullptr;
    CelParser::ExprListContext *args = nullptr;
    MemberContext *member();
    antlr4::tree::TerminalNode *RPAREN();
    antlr4::tree::TerminalNode *DOT();
    antlr4::tree::TerminalNode *IDENTIFIER();
    antlr4::tree::TerminalNode *LPAREN();
    ExprListContext *exprList();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  SelectContext : public MemberContext {
  public:
    SelectContext(MemberContext *ctx);

    antlr4::Token *op = nullptr;
    antlr4::Token *opt = nullptr;
    CelParser::EscapeIdentContext *id = nullptr;
    MemberContext *member();
    antlr4::tree::TerminalNode *DOT();
    EscapeIdentContext *escapeIdent();
    antlr4::tree::TerminalNode *QUESTIONMARK();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  PrimaryExprContext : public MemberContext {
  public:
    PrimaryExprContext(MemberContext *ctx);

    PrimaryContext *primary();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  IndexContext : public MemberContext {
  public:
    IndexContext(MemberContext *ctx);

    antlr4::Token *op = nullptr;
    antlr4::Token *opt = nullptr;
    CelParser::ExprContext *index = nullptr;
    MemberContext *member();
    antlr4::tree::TerminalNode *RPRACKET();
    antlr4::tree::TerminalNode *LBRACKET();
    ExprContext *expr();
    antlr4::tree::TerminalNode *QUESTIONMARK();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  MemberContext* member();
  MemberContext* member(int precedence);
  class  PrimaryContext : public antlr4::ParserRuleContext {
  public:
    PrimaryContext(antlr4::ParserRuleContext *parent, size_t invokingState);
   
    PrimaryContext() = default;
    void copyFrom(PrimaryContext *context);
    using antlr4::ParserRuleContext::copyFrom;

    virtual size_t getRuleIndex() const override;

   
  };

  class  CreateListContext : public PrimaryContext {
  public:
    CreateListContext(PrimaryContext *ctx);

    antlr4::Token *op = nullptr;
    CelParser::ListInitContext *elems = nullptr;
    antlr4::tree::TerminalNode *RPRACKET();
    antlr4::tree::TerminalNode *LBRACKET();
    antlr4::tree::TerminalNode *COMMA();
    ListInitContext *listInit();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  IdentContext : public PrimaryContext {
  public:
    IdentContext(PrimaryContext *ctx);

    antlr4::Token *leadingDot = nullptr;
    antlr4::Token *id = nullptr;
    antlr4::tree::TerminalNode *IDENTIFIER();
    antlr4::tree::TerminalNode *DOT();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  ConstantLiteralContext : public PrimaryContext {
  public:
    ConstantLiteralContext(PrimaryContext *ctx);

    LiteralContext *literal();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  NestedContext : public PrimaryContext {
  public:
    NestedContext(PrimaryContext *ctx);

    CelParser::ExprContext *e = nullptr;
    antlr4::tree::TerminalNode *LPAREN();
    antlr4::tree::TerminalNode *RPAREN();
    ExprContext *expr();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  CreateMessageContext : public PrimaryContext {
  public:
    CreateMessageContext(PrimaryContext *ctx);

    antlr4::Token *leadingDot = nullptr;
    antlr4::Token *identifierToken = nullptr;
    std::vector<antlr4::Token *> ids;
    antlr4::Token *s16 = nullptr;
    std::vector<antlr4::Token *> ops;
    antlr4::Token *op = nullptr;
    CelParser::FieldInitializerListContext *entries = nullptr;
    antlr4::tree::TerminalNode *RBRACE();
    std::vector<antlr4::tree::TerminalNode *> IDENTIFIER();
    antlr4::tree::TerminalNode* IDENTIFIER(size_t i);
    antlr4::tree::TerminalNode *LBRACE();
    antlr4::tree::TerminalNode *COMMA();
    std::vector<antlr4::tree::TerminalNode *> DOT();
    antlr4::tree::TerminalNode* DOT(size_t i);
    FieldInitializerListContext *fieldInitializerList();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  GlobalCallContext : public PrimaryContext {
  public:
    GlobalCallContext(PrimaryContext *ctx);

    antlr4::Token *leadingDot = nullptr;
    antlr4::Token *id = nullptr;
    antlr4::Token *op = nullptr;
    CelParser::ExprListContext *args = nullptr;
    antlr4::tree::TerminalNode *IDENTIFIER();
    antlr4::tree::TerminalNode *RPAREN();
    antlr4::tree::TerminalNode *LPAREN();
    antlr4::tree::TerminalNode *DOT();
    ExprListContext *exprList();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  CreateMapContext : public PrimaryContext {
  public:
    CreateMapContext(PrimaryContext *ctx);

    antlr4::Token *op = nullptr;
    CelParser::MapInitializerListContext *entries = nullptr;
    antlr4::tree::TerminalNode *RBRACE();
    antlr4::tree::TerminalNode *LBRACE();
    antlr4::tree::TerminalNode *COMMA();
    MapInitializerListContext *mapInitializerList();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  PrimaryContext* primary();

  class  ExprListContext : public antlr4::ParserRuleContext {
  public:
    CelParser::ExprContext *exprContext = nullptr;
    std::vector<ExprContext *> e;
    ExprListContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<ExprContext *> expr();
    ExprContext* expr(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COMMA();
    antlr4::tree::TerminalNode* COMMA(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  ExprListContext* exprList();

  class  ListInitContext : public antlr4::ParserRuleContext {
  public:
    CelParser::OptExprContext *optExprContext = nullptr;
    std::vector<OptExprContext *> elems;
    ListInitContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<OptExprContext *> optExpr();
    OptExprContext* optExpr(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COMMA();
    antlr4::tree::TerminalNode* COMMA(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  ListInitContext* listInit();

  class  FieldInitializerListContext : public antlr4::ParserRuleContext {
  public:
    CelParser::OptFieldContext *optFieldContext = nullptr;
    std::vector<OptFieldContext *> fields;
    antlr4::Token *s21 = nullptr;
    std::vector<antlr4::Token *> cols;
    CelParser::ExprContext *exprContext = nullptr;
    std::vector<ExprContext *> values;
    FieldInitializerListContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<OptFieldContext *> optField();
    OptFieldContext* optField(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COLON();
    antlr4::tree::TerminalNode* COLON(size_t i);
    std::vector<ExprContext *> expr();
    ExprContext* expr(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COMMA();
    antlr4::tree::TerminalNode* COMMA(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  FieldInitializerListContext* fieldInitializerList();

  class  OptFieldContext : public antlr4::ParserRuleContext {
  public:
    antlr4::Token *opt = nullptr;
    OptFieldContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    EscapeIdentContext *escapeIdent();
    antlr4::tree::TerminalNode *QUESTIONMARK();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  OptFieldContext* optField();

  class  MapInitializerListContext : public antlr4::ParserRuleContext {
  public:
    CelParser::OptExprContext *optExprContext = nullptr;
    std::vector<OptExprContext *> keys;
    antlr4::Token *s21 = nullptr;
    std::vector<antlr4::Token *> cols;
    CelParser::ExprContext *exprContext = nullptr;
    std::vector<ExprContext *> values;
    MapInitializerListContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    std::vector<OptExprContext *> optExpr();
    OptExprContext* optExpr(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COLON();
    antlr4::tree::TerminalNode* COLON(size_t i);
    std::vector<ExprContext *> expr();
    ExprContext* expr(size_t i);
    std::vector<antlr4::tree::TerminalNode *> COMMA();
    antlr4::tree::TerminalNode* COMMA(size_t i);


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  MapInitializerListContext* mapInitializerList();

  class  EscapeIdentContext : public antlr4::ParserRuleContext {
  public:
    EscapeIdentContext(antlr4::ParserRuleContext *parent, size_t invokingState);
   
    EscapeIdentContext() = default;
    void copyFrom(EscapeIdentContext *context);
    using antlr4::ParserRuleContext::copyFrom;

    virtual size_t getRuleIndex() const override;

   
  };

  class  EscapedIdentifierContext : public EscapeIdentContext {
  public:
    EscapedIdentifierContext(EscapeIdentContext *ctx);

    antlr4::Token *id = nullptr;
    antlr4::tree::TerminalNode *ESC_IDENTIFIER();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  SimpleIdentifierContext : public EscapeIdentContext {
  public:
    SimpleIdentifierContext(EscapeIdentContext *ctx);

    antlr4::Token *id = nullptr;
    antlr4::tree::TerminalNode *IDENTIFIER();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  EscapeIdentContext* escapeIdent();

  class  OptExprContext : public antlr4::ParserRuleContext {
  public:
    antlr4::Token *opt = nullptr;
    CelParser::ExprContext *e = nullptr;
    OptExprContext(antlr4::ParserRuleContext *parent, size_t invokingState);
    virtual size_t getRuleIndex() const override;
    ExprContext *expr();
    antlr4::tree::TerminalNode *QUESTIONMARK();


    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
   
  };

  OptExprContext* optExpr();

  class  LiteralContext : public antlr4::ParserRuleContext {
  public:
    LiteralContext(antlr4::ParserRuleContext *parent, size_t invokingState);
   
    LiteralContext() = default;
    void copyFrom(LiteralContext *context);
    using antlr4::ParserRuleContext::copyFrom;

    virtual size_t getRuleIndex() const override;

   
  };

  class  BytesContext : public LiteralContext {
  public:
    BytesContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *BYTES();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  UintContext : public LiteralContext {
  public:
    UintContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *NUM_UINT();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  NullContext : public LiteralContext {
  public:
    NullContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *NUL();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  BoolFalseContext : public LiteralContext {
  public:
    BoolFalseContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *CEL_FALSE();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  StringContext : public LiteralContext {
  public:
    StringContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *STRING();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  DoubleContext : public LiteralContext {
  public:
    DoubleContext(LiteralContext *ctx);

    antlr4::Token *sign = nullptr;
    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *NUM_FLOAT();
    antlr4::tree::TerminalNode *MINUS();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  BoolTrueContext : public LiteralContext {
  public:
    BoolTrueContext(LiteralContext *ctx);

    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *CEL_TRUE();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  class  IntContext : public LiteralContext {
  public:
    IntContext(LiteralContext *ctx);

    antlr4::Token *sign = nullptr;
    antlr4::Token *tok = nullptr;
    antlr4::tree::TerminalNode *NUM_INT();
    antlr4::tree::TerminalNode *MINUS();

    virtual std::any accept(antlr4::tree::ParseTreeVisitor *visitor) override;
  };

  LiteralContext* literal();


  bool sempred(antlr4::RuleContext *_localctx, size_t ruleIndex, size_t predicateIndex) override;

  bool relationSempred(RelationContext *_localctx, size_t predicateIndex);
  bool calcSempred(CalcContext *_localctx, size_t predicateIndex);
  bool memberSempred(MemberContext *_localctx, size_t predicateIndex);

  // By default the static state used to implement the parser is lazily initialized during the first
  // call to the constructor. You can call this function if you wish to initialize the static state
  // ahead of time.
  static void initialize();

private:
};

}  // namespace cel_parser_internal
