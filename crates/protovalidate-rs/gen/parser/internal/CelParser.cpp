
// Generated from external/cel-cpp+/parser/internal/Cel.g4 by ANTLR 4.13.2


#include "CelVisitor.h"

#include "CelParser.h"


using namespace antlrcpp;
using namespace cel_parser_internal;

using namespace antlr4;

namespace {

struct CelParserStaticData final {
  CelParserStaticData(std::vector<std::string> ruleNames,
                        std::vector<std::string> literalNames,
                        std::vector<std::string> symbolicNames)
      : ruleNames(std::move(ruleNames)), literalNames(std::move(literalNames)),
        symbolicNames(std::move(symbolicNames)),
        vocabulary(this->literalNames, this->symbolicNames) {}

  CelParserStaticData(const CelParserStaticData&) = delete;
  CelParserStaticData(CelParserStaticData&&) = delete;
  CelParserStaticData& operator=(const CelParserStaticData&) = delete;
  CelParserStaticData& operator=(CelParserStaticData&&) = delete;

  std::vector<antlr4::dfa::DFA> decisionToDFA;
  antlr4::atn::PredictionContextCache sharedContextCache;
  const std::vector<std::string> ruleNames;
  const std::vector<std::string> literalNames;
  const std::vector<std::string> symbolicNames;
  const antlr4::dfa::Vocabulary vocabulary;
  antlr4::atn::SerializedATNView serializedATN;
  std::unique_ptr<antlr4::atn::ATN> atn;
};

::antlr4::internal::OnceFlag celParserOnceFlag;
#if ANTLR4_USE_THREAD_LOCAL_CACHE
static thread_local
#endif
std::unique_ptr<CelParserStaticData> celParserStaticData = nullptr;

void celParserInitialize() {
#if ANTLR4_USE_THREAD_LOCAL_CACHE
  if (celParserStaticData != nullptr) {
    return;
  }
#else
  assert(celParserStaticData == nullptr);
#endif
  auto staticData = std::make_unique<CelParserStaticData>(
    std::vector<std::string>{
      "start", "expr", "conditionalOr", "conditionalAnd", "relation", "calc", 
      "unary", "member", "primary", "exprList", "listInit", "fieldInitializerList", 
      "optField", "mapInitializerList", "escapeIdent", "optExpr", "literal"
    },
    std::vector<std::string>{
      "", "'=='", "'!='", "'in'", "'<'", "'<='", "'>='", "'>'", "'&&'", 
      "'||'", "'['", "']'", "'{'", "'}'", "'('", "')'", "'.'", "','", "'-'", 
      "'!'", "'\\u003F'", "':'", "'+'", "'*'", "'/'", "'%'", "'true'", "'false'", 
      "'null'"
    },
    std::vector<std::string>{
      "", "EQUALS", "NOT_EQUALS", "IN", "LESS", "LESS_EQUALS", "GREATER_EQUALS", 
      "GREATER", "LOGICAL_AND", "LOGICAL_OR", "LBRACKET", "RPRACKET", "LBRACE", 
      "RBRACE", "LPAREN", "RPAREN", "DOT", "COMMA", "MINUS", "EXCLAM", "QUESTIONMARK", 
      "COLON", "PLUS", "STAR", "SLASH", "PERCENT", "CEL_TRUE", "CEL_FALSE", 
      "NUL", "WHITESPACE", "COMMENT", "NUM_FLOAT", "NUM_INT", "NUM_UINT", 
      "STRING", "BYTES", "IDENTIFIER", "ESC_IDENTIFIER"
    }
  );
  static const int32_t serializedATNSegment[] = {
  	4,1,37,259,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,2,4,7,4,2,5,7,5,2,6,7,6,2,
  	7,7,7,2,8,7,8,2,9,7,9,2,10,7,10,2,11,7,11,2,12,7,12,2,13,7,13,2,14,7,
  	14,2,15,7,15,2,16,7,16,1,0,1,0,1,0,1,1,1,1,1,1,1,1,1,1,1,1,3,1,44,8,1,
  	1,2,1,2,1,2,5,2,49,8,2,10,2,12,2,52,9,2,1,3,1,3,1,3,5,3,57,8,3,10,3,12,
  	3,60,9,3,1,4,1,4,1,4,1,4,1,4,1,4,5,4,68,8,4,10,4,12,4,71,9,4,1,5,1,5,
  	1,5,1,5,1,5,1,5,1,5,1,5,1,5,5,5,82,8,5,10,5,12,5,85,9,5,1,6,1,6,4,6,89,
  	8,6,11,6,12,6,90,1,6,1,6,4,6,95,8,6,11,6,12,6,96,1,6,3,6,100,8,6,1,7,
  	1,7,1,7,1,7,1,7,1,7,3,7,108,8,7,1,7,1,7,1,7,1,7,1,7,1,7,3,7,116,8,7,1,
  	7,1,7,1,7,1,7,3,7,122,8,7,1,7,1,7,1,7,5,7,127,8,7,10,7,12,7,130,9,7,1,
  	8,3,8,133,8,8,1,8,1,8,3,8,137,8,8,1,8,1,8,1,8,3,8,142,8,8,1,8,1,8,1,8,
  	1,8,1,8,1,8,1,8,3,8,151,8,8,1,8,3,8,154,8,8,1,8,1,8,1,8,3,8,159,8,8,1,
  	8,3,8,162,8,8,1,8,1,8,3,8,166,8,8,1,8,1,8,1,8,5,8,171,8,8,10,8,12,8,174,
  	9,8,1,8,1,8,3,8,178,8,8,1,8,3,8,181,8,8,1,8,1,8,3,8,185,8,8,1,9,1,9,1,
  	9,5,9,190,8,9,10,9,12,9,193,9,9,1,10,1,10,1,10,5,10,198,8,10,10,10,12,
  	10,201,9,10,1,11,1,11,1,11,1,11,1,11,1,11,1,11,1,11,5,11,211,8,11,10,
  	11,12,11,214,9,11,1,12,3,12,217,8,12,1,12,1,12,1,13,1,13,1,13,1,13,1,
  	13,1,13,1,13,1,13,5,13,229,8,13,10,13,12,13,232,9,13,1,14,1,14,3,14,236,
  	8,14,1,15,3,15,239,8,15,1,15,1,15,1,16,3,16,244,8,16,1,16,1,16,1,16,3,
  	16,249,8,16,1,16,1,16,1,16,1,16,1,16,1,16,3,16,257,8,16,1,16,0,3,8,10,
  	14,17,0,2,4,6,8,10,12,14,16,18,20,22,24,26,28,30,32,0,3,1,0,1,7,1,0,23,
  	25,2,0,18,18,22,22,290,0,34,1,0,0,0,2,37,1,0,0,0,4,45,1,0,0,0,6,53,1,
  	0,0,0,8,61,1,0,0,0,10,72,1,0,0,0,12,99,1,0,0,0,14,101,1,0,0,0,16,184,
  	1,0,0,0,18,186,1,0,0,0,20,194,1,0,0,0,22,202,1,0,0,0,24,216,1,0,0,0,26,
  	220,1,0,0,0,28,235,1,0,0,0,30,238,1,0,0,0,32,256,1,0,0,0,34,35,3,2,1,
  	0,35,36,5,0,0,1,36,1,1,0,0,0,37,43,3,4,2,0,38,39,5,20,0,0,39,40,3,4,2,
  	0,40,41,5,21,0,0,41,42,3,2,1,0,42,44,1,0,0,0,43,38,1,0,0,0,43,44,1,0,
  	0,0,44,3,1,0,0,0,45,50,3,6,3,0,46,47,5,9,0,0,47,49,3,6,3,0,48,46,1,0,
  	0,0,49,52,1,0,0,0,50,48,1,0,0,0,50,51,1,0,0,0,51,5,1,0,0,0,52,50,1,0,
  	0,0,53,58,3,8,4,0,54,55,5,8,0,0,55,57,3,8,4,0,56,54,1,0,0,0,57,60,1,0,
  	0,0,58,56,1,0,0,0,58,59,1,0,0,0,59,7,1,0,0,0,60,58,1,0,0,0,61,62,6,4,
  	-1,0,62,63,3,10,5,0,63,69,1,0,0,0,64,65,10,1,0,0,65,66,7,0,0,0,66,68,
  	3,8,4,2,67,64,1,0,0,0,68,71,1,0,0,0,69,67,1,0,0,0,69,70,1,0,0,0,70,9,
  	1,0,0,0,71,69,1,0,0,0,72,73,6,5,-1,0,73,74,3,12,6,0,74,83,1,0,0,0,75,
  	76,10,2,0,0,76,77,7,1,0,0,77,82,3,10,5,3,78,79,10,1,0,0,79,80,7,2,0,0,
  	80,82,3,10,5,2,81,75,1,0,0,0,81,78,1,0,0,0,82,85,1,0,0,0,83,81,1,0,0,
  	0,83,84,1,0,0,0,84,11,1,0,0,0,85,83,1,0,0,0,86,100,3,14,7,0,87,89,5,19,
  	0,0,88,87,1,0,0,0,89,90,1,0,0,0,90,88,1,0,0,0,90,91,1,0,0,0,91,92,1,0,
  	0,0,92,100,3,14,7,0,93,95,5,18,0,0,94,93,1,0,0,0,95,96,1,0,0,0,96,94,
  	1,0,0,0,96,97,1,0,0,0,97,98,1,0,0,0,98,100,3,14,7,0,99,86,1,0,0,0,99,
  	88,1,0,0,0,99,94,1,0,0,0,100,13,1,0,0,0,101,102,6,7,-1,0,102,103,3,16,
  	8,0,103,128,1,0,0,0,104,105,10,3,0,0,105,107,5,16,0,0,106,108,5,20,0,
  	0,107,106,1,0,0,0,107,108,1,0,0,0,108,109,1,0,0,0,109,127,3,28,14,0,110,
  	111,10,2,0,0,111,112,5,16,0,0,112,113,5,36,0,0,113,115,5,14,0,0,114,116,
  	3,18,9,0,115,114,1,0,0,0,115,116,1,0,0,0,116,117,1,0,0,0,117,127,5,15,
  	0,0,118,119,10,1,0,0,119,121,5,10,0,0,120,122,5,20,0,0,121,120,1,0,0,
  	0,121,122,1,0,0,0,122,123,1,0,0,0,123,124,3,2,1,0,124,125,5,11,0,0,125,
  	127,1,0,0,0,126,104,1,0,0,0,126,110,1,0,0,0,126,118,1,0,0,0,127,130,1,
  	0,0,0,128,126,1,0,0,0,128,129,1,0,0,0,129,15,1,0,0,0,130,128,1,0,0,0,
  	131,133,5,16,0,0,132,131,1,0,0,0,132,133,1,0,0,0,133,134,1,0,0,0,134,
  	185,5,36,0,0,135,137,5,16,0,0,136,135,1,0,0,0,136,137,1,0,0,0,137,138,
  	1,0,0,0,138,139,5,36,0,0,139,141,5,14,0,0,140,142,3,18,9,0,141,140,1,
  	0,0,0,141,142,1,0,0,0,142,143,1,0,0,0,143,185,5,15,0,0,144,145,5,14,0,
  	0,145,146,3,2,1,0,146,147,5,15,0,0,147,185,1,0,0,0,148,150,5,10,0,0,149,
  	151,3,20,10,0,150,149,1,0,0,0,150,151,1,0,0,0,151,153,1,0,0,0,152,154,
  	5,17,0,0,153,152,1,0,0,0,153,154,1,0,0,0,154,155,1,0,0,0,155,185,5,11,
  	0,0,156,158,5,12,0,0,157,159,3,26,13,0,158,157,1,0,0,0,158,159,1,0,0,
  	0,159,161,1,0,0,0,160,162,5,17,0,0,161,160,1,0,0,0,161,162,1,0,0,0,162,
  	163,1,0,0,0,163,185,5,13,0,0,164,166,5,16,0,0,165,164,1,0,0,0,165,166,
  	1,0,0,0,166,167,1,0,0,0,167,172,5,36,0,0,168,169,5,16,0,0,169,171,5,36,
  	0,0,170,168,1,0,0,0,171,174,1,0,0,0,172,170,1,0,0,0,172,173,1,0,0,0,173,
  	175,1,0,0,0,174,172,1,0,0,0,175,177,5,12,0,0,176,178,3,22,11,0,177,176,
  	1,0,0,0,177,178,1,0,0,0,178,180,1,0,0,0,179,181,5,17,0,0,180,179,1,0,
  	0,0,180,181,1,0,0,0,181,182,1,0,0,0,182,185,5,13,0,0,183,185,3,32,16,
  	0,184,132,1,0,0,0,184,136,1,0,0,0,184,144,1,0,0,0,184,148,1,0,0,0,184,
  	156,1,0,0,0,184,165,1,0,0,0,184,183,1,0,0,0,185,17,1,0,0,0,186,191,3,
  	2,1,0,187,188,5,17,0,0,188,190,3,2,1,0,189,187,1,0,0,0,190,193,1,0,0,
  	0,191,189,1,0,0,0,191,192,1,0,0,0,192,19,1,0,0,0,193,191,1,0,0,0,194,
  	199,3,30,15,0,195,196,5,17,0,0,196,198,3,30,15,0,197,195,1,0,0,0,198,
  	201,1,0,0,0,199,197,1,0,0,0,199,200,1,0,0,0,200,21,1,0,0,0,201,199,1,
  	0,0,0,202,203,3,24,12,0,203,204,5,21,0,0,204,212,3,2,1,0,205,206,5,17,
  	0,0,206,207,3,24,12,0,207,208,5,21,0,0,208,209,3,2,1,0,209,211,1,0,0,
  	0,210,205,1,0,0,0,211,214,1,0,0,0,212,210,1,0,0,0,212,213,1,0,0,0,213,
  	23,1,0,0,0,214,212,1,0,0,0,215,217,5,20,0,0,216,215,1,0,0,0,216,217,1,
  	0,0,0,217,218,1,0,0,0,218,219,3,28,14,0,219,25,1,0,0,0,220,221,3,30,15,
  	0,221,222,5,21,0,0,222,230,3,2,1,0,223,224,5,17,0,0,224,225,3,30,15,0,
  	225,226,5,21,0,0,226,227,3,2,1,0,227,229,1,0,0,0,228,223,1,0,0,0,229,
  	232,1,0,0,0,230,228,1,0,0,0,230,231,1,0,0,0,231,27,1,0,0,0,232,230,1,
  	0,0,0,233,236,5,36,0,0,234,236,5,37,0,0,235,233,1,0,0,0,235,234,1,0,0,
  	0,236,29,1,0,0,0,237,239,5,20,0,0,238,237,1,0,0,0,238,239,1,0,0,0,239,
  	240,1,0,0,0,240,241,3,2,1,0,241,31,1,0,0,0,242,244,5,18,0,0,243,242,1,
  	0,0,0,243,244,1,0,0,0,244,245,1,0,0,0,245,257,5,32,0,0,246,257,5,33,0,
  	0,247,249,5,18,0,0,248,247,1,0,0,0,248,249,1,0,0,0,249,250,1,0,0,0,250,
  	257,5,31,0,0,251,257,5,34,0,0,252,257,5,35,0,0,253,257,5,26,0,0,254,257,
  	5,27,0,0,255,257,5,28,0,0,256,243,1,0,0,0,256,246,1,0,0,0,256,248,1,0,
  	0,0,256,251,1,0,0,0,256,252,1,0,0,0,256,253,1,0,0,0,256,254,1,0,0,0,256,
  	255,1,0,0,0,257,33,1,0,0,0,36,43,50,58,69,81,83,90,96,99,107,115,121,
  	126,128,132,136,141,150,153,158,161,165,172,177,180,184,191,199,212,216,
  	230,235,238,243,248,256
  };
  staticData->serializedATN = antlr4::atn::SerializedATNView(serializedATNSegment, sizeof(serializedATNSegment) / sizeof(serializedATNSegment[0]));

  antlr4::atn::ATNDeserializer deserializer;
  staticData->atn = deserializer.deserialize(staticData->serializedATN);

  const size_t count = staticData->atn->getNumberOfDecisions();
  staticData->decisionToDFA.reserve(count);
  for (size_t i = 0; i < count; i++) { 
    staticData->decisionToDFA.emplace_back(staticData->atn->getDecisionState(i), i);
  }
  celParserStaticData = std::move(staticData);
}

}

CelParser::CelParser(TokenStream *input) : CelParser(input, antlr4::atn::ParserATNSimulatorOptions()) {}

CelParser::CelParser(TokenStream *input, const antlr4::atn::ParserATNSimulatorOptions &options) : Parser(input) {
  CelParser::initialize();
  _interpreter = new atn::ParserATNSimulator(this, *celParserStaticData->atn, celParserStaticData->decisionToDFA, celParserStaticData->sharedContextCache, options);
}

CelParser::~CelParser() {
  delete _interpreter;
}

const atn::ATN& CelParser::getATN() const {
  return *celParserStaticData->atn;
}

std::string CelParser::getGrammarFileName() const {
  return "Cel.g4";
}

const std::vector<std::string>& CelParser::getRuleNames() const {
  return celParserStaticData->ruleNames;
}

const dfa::Vocabulary& CelParser::getVocabulary() const {
  return celParserStaticData->vocabulary;
}

antlr4::atn::SerializedATNView CelParser::getSerializedATN() const {
  return celParserStaticData->serializedATN;
}


//----------------- StartContext ------------------------------------------------------------------

CelParser::StartContext::StartContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

tree::TerminalNode* CelParser::StartContext::EOF() {
  return getToken(CelParser::EOF, 0);
}

CelParser::ExprContext* CelParser::StartContext::expr() {
  return getRuleContext<CelParser::ExprContext>(0);
}


size_t CelParser::StartContext::getRuleIndex() const {
  return CelParser::RuleStart;
}


std::any CelParser::StartContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitStart(this);
  else
    return visitor->visitChildren(this);
}

CelParser::StartContext* CelParser::start() {
  StartContext *_localctx = _tracker.createInstance<StartContext>(_ctx, getState());
  enterRule(_localctx, 0, CelParser::RuleStart);

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(34);
    antlrcpp::downCast<StartContext *>(_localctx)->e = expr();
    setState(35);
    match(CelParser::EOF);
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- ExprContext ------------------------------------------------------------------

CelParser::ExprContext::ExprContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::ConditionalOrContext *> CelParser::ExprContext::conditionalOr() {
  return getRuleContexts<CelParser::ConditionalOrContext>();
}

CelParser::ConditionalOrContext* CelParser::ExprContext::conditionalOr(size_t i) {
  return getRuleContext<CelParser::ConditionalOrContext>(i);
}

tree::TerminalNode* CelParser::ExprContext::COLON() {
  return getToken(CelParser::COLON, 0);
}

tree::TerminalNode* CelParser::ExprContext::QUESTIONMARK() {
  return getToken(CelParser::QUESTIONMARK, 0);
}

CelParser::ExprContext* CelParser::ExprContext::expr() {
  return getRuleContext<CelParser::ExprContext>(0);
}


size_t CelParser::ExprContext::getRuleIndex() const {
  return CelParser::RuleExpr;
}


std::any CelParser::ExprContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitExpr(this);
  else
    return visitor->visitChildren(this);
}

CelParser::ExprContext* CelParser::expr() {
  ExprContext *_localctx = _tracker.createInstance<ExprContext>(_ctx, getState());
  enterRule(_localctx, 2, CelParser::RuleExpr);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(37);
    antlrcpp::downCast<ExprContext *>(_localctx)->e = conditionalOr();
    setState(43);
    _errHandler->sync(this);

    _la = _input->LA(1);
    if (_la == CelParser::QUESTIONMARK) {
      setState(38);
      antlrcpp::downCast<ExprContext *>(_localctx)->op = match(CelParser::QUESTIONMARK);
      setState(39);
      antlrcpp::downCast<ExprContext *>(_localctx)->e1 = conditionalOr();
      setState(40);
      match(CelParser::COLON);
      setState(41);
      antlrcpp::downCast<ExprContext *>(_localctx)->e2 = expr();
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- ConditionalOrContext ------------------------------------------------------------------

CelParser::ConditionalOrContext::ConditionalOrContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::ConditionalAndContext *> CelParser::ConditionalOrContext::conditionalAnd() {
  return getRuleContexts<CelParser::ConditionalAndContext>();
}

CelParser::ConditionalAndContext* CelParser::ConditionalOrContext::conditionalAnd(size_t i) {
  return getRuleContext<CelParser::ConditionalAndContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::ConditionalOrContext::LOGICAL_OR() {
  return getTokens(CelParser::LOGICAL_OR);
}

tree::TerminalNode* CelParser::ConditionalOrContext::LOGICAL_OR(size_t i) {
  return getToken(CelParser::LOGICAL_OR, i);
}


size_t CelParser::ConditionalOrContext::getRuleIndex() const {
  return CelParser::RuleConditionalOr;
}


std::any CelParser::ConditionalOrContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitConditionalOr(this);
  else
    return visitor->visitChildren(this);
}

CelParser::ConditionalOrContext* CelParser::conditionalOr() {
  ConditionalOrContext *_localctx = _tracker.createInstance<ConditionalOrContext>(_ctx, getState());
  enterRule(_localctx, 4, CelParser::RuleConditionalOr);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(45);
    antlrcpp::downCast<ConditionalOrContext *>(_localctx)->e = conditionalAnd();
    setState(50);
    _errHandler->sync(this);
    _la = _input->LA(1);
    while (_la == CelParser::LOGICAL_OR) {
      setState(46);
      antlrcpp::downCast<ConditionalOrContext *>(_localctx)->s9 = match(CelParser::LOGICAL_OR);
      antlrcpp::downCast<ConditionalOrContext *>(_localctx)->ops.push_back(antlrcpp::downCast<ConditionalOrContext *>(_localctx)->s9);
      setState(47);
      antlrcpp::downCast<ConditionalOrContext *>(_localctx)->conditionalAndContext = conditionalAnd();
      antlrcpp::downCast<ConditionalOrContext *>(_localctx)->e1.push_back(antlrcpp::downCast<ConditionalOrContext *>(_localctx)->conditionalAndContext);
      setState(52);
      _errHandler->sync(this);
      _la = _input->LA(1);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- ConditionalAndContext ------------------------------------------------------------------

CelParser::ConditionalAndContext::ConditionalAndContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::RelationContext *> CelParser::ConditionalAndContext::relation() {
  return getRuleContexts<CelParser::RelationContext>();
}

CelParser::RelationContext* CelParser::ConditionalAndContext::relation(size_t i) {
  return getRuleContext<CelParser::RelationContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::ConditionalAndContext::LOGICAL_AND() {
  return getTokens(CelParser::LOGICAL_AND);
}

tree::TerminalNode* CelParser::ConditionalAndContext::LOGICAL_AND(size_t i) {
  return getToken(CelParser::LOGICAL_AND, i);
}


size_t CelParser::ConditionalAndContext::getRuleIndex() const {
  return CelParser::RuleConditionalAnd;
}


std::any CelParser::ConditionalAndContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitConditionalAnd(this);
  else
    return visitor->visitChildren(this);
}

CelParser::ConditionalAndContext* CelParser::conditionalAnd() {
  ConditionalAndContext *_localctx = _tracker.createInstance<ConditionalAndContext>(_ctx, getState());
  enterRule(_localctx, 6, CelParser::RuleConditionalAnd);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(53);
    antlrcpp::downCast<ConditionalAndContext *>(_localctx)->e = relation(0);
    setState(58);
    _errHandler->sync(this);
    _la = _input->LA(1);
    while (_la == CelParser::LOGICAL_AND) {
      setState(54);
      antlrcpp::downCast<ConditionalAndContext *>(_localctx)->s8 = match(CelParser::LOGICAL_AND);
      antlrcpp::downCast<ConditionalAndContext *>(_localctx)->ops.push_back(antlrcpp::downCast<ConditionalAndContext *>(_localctx)->s8);
      setState(55);
      antlrcpp::downCast<ConditionalAndContext *>(_localctx)->relationContext = relation(0);
      antlrcpp::downCast<ConditionalAndContext *>(_localctx)->e1.push_back(antlrcpp::downCast<ConditionalAndContext *>(_localctx)->relationContext);
      setState(60);
      _errHandler->sync(this);
      _la = _input->LA(1);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- RelationContext ------------------------------------------------------------------

CelParser::RelationContext::RelationContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

CelParser::CalcContext* CelParser::RelationContext::calc() {
  return getRuleContext<CelParser::CalcContext>(0);
}

std::vector<CelParser::RelationContext *> CelParser::RelationContext::relation() {
  return getRuleContexts<CelParser::RelationContext>();
}

CelParser::RelationContext* CelParser::RelationContext::relation(size_t i) {
  return getRuleContext<CelParser::RelationContext>(i);
}

tree::TerminalNode* CelParser::RelationContext::LESS() {
  return getToken(CelParser::LESS, 0);
}

tree::TerminalNode* CelParser::RelationContext::LESS_EQUALS() {
  return getToken(CelParser::LESS_EQUALS, 0);
}

tree::TerminalNode* CelParser::RelationContext::GREATER_EQUALS() {
  return getToken(CelParser::GREATER_EQUALS, 0);
}

tree::TerminalNode* CelParser::RelationContext::GREATER() {
  return getToken(CelParser::GREATER, 0);
}

tree::TerminalNode* CelParser::RelationContext::EQUALS() {
  return getToken(CelParser::EQUALS, 0);
}

tree::TerminalNode* CelParser::RelationContext::NOT_EQUALS() {
  return getToken(CelParser::NOT_EQUALS, 0);
}

tree::TerminalNode* CelParser::RelationContext::IN() {
  return getToken(CelParser::IN, 0);
}


size_t CelParser::RelationContext::getRuleIndex() const {
  return CelParser::RuleRelation;
}


std::any CelParser::RelationContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitRelation(this);
  else
    return visitor->visitChildren(this);
}


CelParser::RelationContext* CelParser::relation() {
   return relation(0);
}

CelParser::RelationContext* CelParser::relation(int precedence) {
  ParserRuleContext *parentContext = _ctx;
  size_t parentState = getState();
  CelParser::RelationContext *_localctx = _tracker.createInstance<RelationContext>(_ctx, parentState);
  CelParser::RelationContext *previousContext = _localctx;
  (void)previousContext; // Silence compiler, in case the context is not used by generated code.
  size_t startState = 8;
  enterRecursionRule(_localctx, 8, CelParser::RuleRelation, precedence);

    size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    unrollRecursionContexts(parentContext);
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    setState(62);
    calc(0);
    _ctx->stop = _input->LT(-1);
    setState(69);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 3, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        if (!_parseListeners.empty())
          triggerExitRuleEvent();
        previousContext = _localctx;
        _localctx = _tracker.createInstance<RelationContext>(parentContext, parentState);
        pushNewRecursionContext(_localctx, startState, RuleRelation);
        setState(64);

        if (!(precpred(_ctx, 1))) throw FailedPredicateException(this, "precpred(_ctx, 1)");
        setState(65);
        antlrcpp::downCast<RelationContext *>(_localctx)->op = _input->LT(1);
        _la = _input->LA(1);
        if (!((((_la & ~ 0x3fULL) == 0) &&
          ((1ULL << _la) & 254) != 0))) {
          antlrcpp::downCast<RelationContext *>(_localctx)->op = _errHandler->recoverInline(this);
        }
        else {
          _errHandler->reportMatch(this);
          consume();
        }
        setState(66);
        relation(2); 
      }
      setState(71);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 3, _ctx);
    }
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }
  return _localctx;
}

//----------------- CalcContext ------------------------------------------------------------------

CelParser::CalcContext::CalcContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

CelParser::UnaryContext* CelParser::CalcContext::unary() {
  return getRuleContext<CelParser::UnaryContext>(0);
}

std::vector<CelParser::CalcContext *> CelParser::CalcContext::calc() {
  return getRuleContexts<CelParser::CalcContext>();
}

CelParser::CalcContext* CelParser::CalcContext::calc(size_t i) {
  return getRuleContext<CelParser::CalcContext>(i);
}

tree::TerminalNode* CelParser::CalcContext::STAR() {
  return getToken(CelParser::STAR, 0);
}

tree::TerminalNode* CelParser::CalcContext::SLASH() {
  return getToken(CelParser::SLASH, 0);
}

tree::TerminalNode* CelParser::CalcContext::PERCENT() {
  return getToken(CelParser::PERCENT, 0);
}

tree::TerminalNode* CelParser::CalcContext::PLUS() {
  return getToken(CelParser::PLUS, 0);
}

tree::TerminalNode* CelParser::CalcContext::MINUS() {
  return getToken(CelParser::MINUS, 0);
}


size_t CelParser::CalcContext::getRuleIndex() const {
  return CelParser::RuleCalc;
}


std::any CelParser::CalcContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitCalc(this);
  else
    return visitor->visitChildren(this);
}


CelParser::CalcContext* CelParser::calc() {
   return calc(0);
}

CelParser::CalcContext* CelParser::calc(int precedence) {
  ParserRuleContext *parentContext = _ctx;
  size_t parentState = getState();
  CelParser::CalcContext *_localctx = _tracker.createInstance<CalcContext>(_ctx, parentState);
  CelParser::CalcContext *previousContext = _localctx;
  (void)previousContext; // Silence compiler, in case the context is not used by generated code.
  size_t startState = 10;
  enterRecursionRule(_localctx, 10, CelParser::RuleCalc, precedence);

    size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    unrollRecursionContexts(parentContext);
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    setState(73);
    unary();
    _ctx->stop = _input->LT(-1);
    setState(83);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 5, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        if (!_parseListeners.empty())
          triggerExitRuleEvent();
        previousContext = _localctx;
        setState(81);
        _errHandler->sync(this);
        switch (getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 4, _ctx)) {
        case 1: {
          _localctx = _tracker.createInstance<CalcContext>(parentContext, parentState);
          pushNewRecursionContext(_localctx, startState, RuleCalc);
          setState(75);

          if (!(precpred(_ctx, 2))) throw FailedPredicateException(this, "precpred(_ctx, 2)");
          setState(76);
          antlrcpp::downCast<CalcContext *>(_localctx)->op = _input->LT(1);
          _la = _input->LA(1);
          if (!((((_la & ~ 0x3fULL) == 0) &&
            ((1ULL << _la) & 58720256) != 0))) {
            antlrcpp::downCast<CalcContext *>(_localctx)->op = _errHandler->recoverInline(this);
          }
          else {
            _errHandler->reportMatch(this);
            consume();
          }
          setState(77);
          calc(3);
          break;
        }

        case 2: {
          _localctx = _tracker.createInstance<CalcContext>(parentContext, parentState);
          pushNewRecursionContext(_localctx, startState, RuleCalc);
          setState(78);

          if (!(precpred(_ctx, 1))) throw FailedPredicateException(this, "precpred(_ctx, 1)");
          setState(79);
          antlrcpp::downCast<CalcContext *>(_localctx)->op = _input->LT(1);
          _la = _input->LA(1);
          if (!(_la == CelParser::MINUS

          || _la == CelParser::PLUS)) {
            antlrcpp::downCast<CalcContext *>(_localctx)->op = _errHandler->recoverInline(this);
          }
          else {
            _errHandler->reportMatch(this);
            consume();
          }
          setState(80);
          calc(2);
          break;
        }

        default:
          break;
        } 
      }
      setState(85);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 5, _ctx);
    }
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }
  return _localctx;
}

//----------------- UnaryContext ------------------------------------------------------------------

CelParser::UnaryContext::UnaryContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}


size_t CelParser::UnaryContext::getRuleIndex() const {
  return CelParser::RuleUnary;
}

void CelParser::UnaryContext::copyFrom(UnaryContext *ctx) {
  ParserRuleContext::copyFrom(ctx);
}

//----------------- LogicalNotContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::LogicalNotContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

std::vector<tree::TerminalNode *> CelParser::LogicalNotContext::EXCLAM() {
  return getTokens(CelParser::EXCLAM);
}

tree::TerminalNode* CelParser::LogicalNotContext::EXCLAM(size_t i) {
  return getToken(CelParser::EXCLAM, i);
}

CelParser::LogicalNotContext::LogicalNotContext(UnaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::LogicalNotContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitLogicalNot(this);
  else
    return visitor->visitChildren(this);
}
//----------------- MemberExprContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::MemberExprContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

CelParser::MemberExprContext::MemberExprContext(UnaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::MemberExprContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitMemberExpr(this);
  else
    return visitor->visitChildren(this);
}
//----------------- NegateContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::NegateContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

std::vector<tree::TerminalNode *> CelParser::NegateContext::MINUS() {
  return getTokens(CelParser::MINUS);
}

tree::TerminalNode* CelParser::NegateContext::MINUS(size_t i) {
  return getToken(CelParser::MINUS, i);
}

CelParser::NegateContext::NegateContext(UnaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::NegateContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitNegate(this);
  else
    return visitor->visitChildren(this);
}
CelParser::UnaryContext* CelParser::unary() {
  UnaryContext *_localctx = _tracker.createInstance<UnaryContext>(_ctx, getState());
  enterRule(_localctx, 12, CelParser::RuleUnary);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    size_t alt;
    setState(99);
    _errHandler->sync(this);
    switch (getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 8, _ctx)) {
    case 1: {
      _localctx = _tracker.createInstance<CelParser::MemberExprContext>(_localctx);
      enterOuterAlt(_localctx, 1);
      setState(86);
      member(0);
      break;
    }

    case 2: {
      _localctx = _tracker.createInstance<CelParser::LogicalNotContext>(_localctx);
      enterOuterAlt(_localctx, 2);
      setState(88); 
      _errHandler->sync(this);
      _la = _input->LA(1);
      do {
        setState(87);
        antlrcpp::downCast<LogicalNotContext *>(_localctx)->s19 = match(CelParser::EXCLAM);
        antlrcpp::downCast<LogicalNotContext *>(_localctx)->ops.push_back(antlrcpp::downCast<LogicalNotContext *>(_localctx)->s19);
        setState(90); 
        _errHandler->sync(this);
        _la = _input->LA(1);
      } while (_la == CelParser::EXCLAM);
      setState(92);
      member(0);
      break;
    }

    case 3: {
      _localctx = _tracker.createInstance<CelParser::NegateContext>(_localctx);
      enterOuterAlt(_localctx, 3);
      setState(94); 
      _errHandler->sync(this);
      alt = 1;
      do {
        switch (alt) {
          case 1: {
                setState(93);
                antlrcpp::downCast<NegateContext *>(_localctx)->s18 = match(CelParser::MINUS);
                antlrcpp::downCast<NegateContext *>(_localctx)->ops.push_back(antlrcpp::downCast<NegateContext *>(_localctx)->s18);
                break;
              }

        default:
          throw NoViableAltException(this);
        }
        setState(96); 
        _errHandler->sync(this);
        alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 7, _ctx);
      } while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER);
      setState(98);
      member(0);
      break;
    }

    default:
      break;
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- MemberContext ------------------------------------------------------------------

CelParser::MemberContext::MemberContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}


size_t CelParser::MemberContext::getRuleIndex() const {
  return CelParser::RuleMember;
}

void CelParser::MemberContext::copyFrom(MemberContext *ctx) {
  ParserRuleContext::copyFrom(ctx);
}

//----------------- MemberCallContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::MemberCallContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

tree::TerminalNode* CelParser::MemberCallContext::RPAREN() {
  return getToken(CelParser::RPAREN, 0);
}

tree::TerminalNode* CelParser::MemberCallContext::DOT() {
  return getToken(CelParser::DOT, 0);
}

tree::TerminalNode* CelParser::MemberCallContext::IDENTIFIER() {
  return getToken(CelParser::IDENTIFIER, 0);
}

tree::TerminalNode* CelParser::MemberCallContext::LPAREN() {
  return getToken(CelParser::LPAREN, 0);
}

CelParser::ExprListContext* CelParser::MemberCallContext::exprList() {
  return getRuleContext<CelParser::ExprListContext>(0);
}

CelParser::MemberCallContext::MemberCallContext(MemberContext *ctx) { copyFrom(ctx); }


std::any CelParser::MemberCallContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitMemberCall(this);
  else
    return visitor->visitChildren(this);
}
//----------------- SelectContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::SelectContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

tree::TerminalNode* CelParser::SelectContext::DOT() {
  return getToken(CelParser::DOT, 0);
}

CelParser::EscapeIdentContext* CelParser::SelectContext::escapeIdent() {
  return getRuleContext<CelParser::EscapeIdentContext>(0);
}

tree::TerminalNode* CelParser::SelectContext::QUESTIONMARK() {
  return getToken(CelParser::QUESTIONMARK, 0);
}

CelParser::SelectContext::SelectContext(MemberContext *ctx) { copyFrom(ctx); }


std::any CelParser::SelectContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitSelect(this);
  else
    return visitor->visitChildren(this);
}
//----------------- PrimaryExprContext ------------------------------------------------------------------

CelParser::PrimaryContext* CelParser::PrimaryExprContext::primary() {
  return getRuleContext<CelParser::PrimaryContext>(0);
}

CelParser::PrimaryExprContext::PrimaryExprContext(MemberContext *ctx) { copyFrom(ctx); }


std::any CelParser::PrimaryExprContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitPrimaryExpr(this);
  else
    return visitor->visitChildren(this);
}
//----------------- IndexContext ------------------------------------------------------------------

CelParser::MemberContext* CelParser::IndexContext::member() {
  return getRuleContext<CelParser::MemberContext>(0);
}

tree::TerminalNode* CelParser::IndexContext::RPRACKET() {
  return getToken(CelParser::RPRACKET, 0);
}

tree::TerminalNode* CelParser::IndexContext::LBRACKET() {
  return getToken(CelParser::LBRACKET, 0);
}

CelParser::ExprContext* CelParser::IndexContext::expr() {
  return getRuleContext<CelParser::ExprContext>(0);
}

tree::TerminalNode* CelParser::IndexContext::QUESTIONMARK() {
  return getToken(CelParser::QUESTIONMARK, 0);
}

CelParser::IndexContext::IndexContext(MemberContext *ctx) { copyFrom(ctx); }


std::any CelParser::IndexContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitIndex(this);
  else
    return visitor->visitChildren(this);
}

CelParser::MemberContext* CelParser::member() {
   return member(0);
}

CelParser::MemberContext* CelParser::member(int precedence) {
  ParserRuleContext *parentContext = _ctx;
  size_t parentState = getState();
  CelParser::MemberContext *_localctx = _tracker.createInstance<MemberContext>(_ctx, parentState);
  CelParser::MemberContext *previousContext = _localctx;
  (void)previousContext; // Silence compiler, in case the context is not used by generated code.
  size_t startState = 14;
  enterRecursionRule(_localctx, 14, CelParser::RuleMember, precedence);

    size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    unrollRecursionContexts(parentContext);
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    _localctx = _tracker.createInstance<PrimaryExprContext>(_localctx);
    _ctx = _localctx;
    previousContext = _localctx;

    setState(102);
    primary();
    _ctx->stop = _input->LT(-1);
    setState(128);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 13, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        if (!_parseListeners.empty())
          triggerExitRuleEvent();
        previousContext = _localctx;
        setState(126);
        _errHandler->sync(this);
        switch (getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 12, _ctx)) {
        case 1: {
          auto newContext = _tracker.createInstance<SelectContext>(_tracker.createInstance<MemberContext>(parentContext, parentState));
          _localctx = newContext;
          pushNewRecursionContext(newContext, startState, RuleMember);
          setState(104);

          if (!(precpred(_ctx, 3))) throw FailedPredicateException(this, "precpred(_ctx, 3)");
          setState(105);
          antlrcpp::downCast<SelectContext *>(_localctx)->op = match(CelParser::DOT);
          setState(107);
          _errHandler->sync(this);

          _la = _input->LA(1);
          if (_la == CelParser::QUESTIONMARK) {
            setState(106);
            antlrcpp::downCast<SelectContext *>(_localctx)->opt = match(CelParser::QUESTIONMARK);
          }
          setState(109);
          antlrcpp::downCast<SelectContext *>(_localctx)->id = escapeIdent();
          break;
        }

        case 2: {
          auto newContext = _tracker.createInstance<MemberCallContext>(_tracker.createInstance<MemberContext>(parentContext, parentState));
          _localctx = newContext;
          pushNewRecursionContext(newContext, startState, RuleMember);
          setState(110);

          if (!(precpred(_ctx, 2))) throw FailedPredicateException(this, "precpred(_ctx, 2)");
          setState(111);
          antlrcpp::downCast<MemberCallContext *>(_localctx)->op = match(CelParser::DOT);
          setState(112);
          antlrcpp::downCast<MemberCallContext *>(_localctx)->id = match(CelParser::IDENTIFIER);
          setState(113);
          antlrcpp::downCast<MemberCallContext *>(_localctx)->open = match(CelParser::LPAREN);
          setState(115);
          _errHandler->sync(this);

          _la = _input->LA(1);
          if ((((_la & ~ 0x3fULL) == 0) &&
            ((1ULL << _la) & 135762105344) != 0)) {
            setState(114);
            antlrcpp::downCast<MemberCallContext *>(_localctx)->args = exprList();
          }
          setState(117);
          match(CelParser::RPAREN);
          break;
        }

        case 3: {
          auto newContext = _tracker.createInstance<IndexContext>(_tracker.createInstance<MemberContext>(parentContext, parentState));
          _localctx = newContext;
          pushNewRecursionContext(newContext, startState, RuleMember);
          setState(118);

          if (!(precpred(_ctx, 1))) throw FailedPredicateException(this, "precpred(_ctx, 1)");
          setState(119);
          antlrcpp::downCast<IndexContext *>(_localctx)->op = match(CelParser::LBRACKET);
          setState(121);
          _errHandler->sync(this);

          _la = _input->LA(1);
          if (_la == CelParser::QUESTIONMARK) {
            setState(120);
            antlrcpp::downCast<IndexContext *>(_localctx)->opt = match(CelParser::QUESTIONMARK);
          }
          setState(123);
          antlrcpp::downCast<IndexContext *>(_localctx)->index = expr();
          setState(124);
          match(CelParser::RPRACKET);
          break;
        }

        default:
          break;
        } 
      }
      setState(130);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 13, _ctx);
    }
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }
  return _localctx;
}

//----------------- PrimaryContext ------------------------------------------------------------------

CelParser::PrimaryContext::PrimaryContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}


size_t CelParser::PrimaryContext::getRuleIndex() const {
  return CelParser::RulePrimary;
}

void CelParser::PrimaryContext::copyFrom(PrimaryContext *ctx) {
  ParserRuleContext::copyFrom(ctx);
}

//----------------- CreateListContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::CreateListContext::RPRACKET() {
  return getToken(CelParser::RPRACKET, 0);
}

tree::TerminalNode* CelParser::CreateListContext::LBRACKET() {
  return getToken(CelParser::LBRACKET, 0);
}

tree::TerminalNode* CelParser::CreateListContext::COMMA() {
  return getToken(CelParser::COMMA, 0);
}

CelParser::ListInitContext* CelParser::CreateListContext::listInit() {
  return getRuleContext<CelParser::ListInitContext>(0);
}

CelParser::CreateListContext::CreateListContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::CreateListContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitCreateList(this);
  else
    return visitor->visitChildren(this);
}
//----------------- IdentContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::IdentContext::IDENTIFIER() {
  return getToken(CelParser::IDENTIFIER, 0);
}

tree::TerminalNode* CelParser::IdentContext::DOT() {
  return getToken(CelParser::DOT, 0);
}

CelParser::IdentContext::IdentContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::IdentContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitIdent(this);
  else
    return visitor->visitChildren(this);
}
//----------------- ConstantLiteralContext ------------------------------------------------------------------

CelParser::LiteralContext* CelParser::ConstantLiteralContext::literal() {
  return getRuleContext<CelParser::LiteralContext>(0);
}

CelParser::ConstantLiteralContext::ConstantLiteralContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::ConstantLiteralContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitConstantLiteral(this);
  else
    return visitor->visitChildren(this);
}
//----------------- NestedContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::NestedContext::LPAREN() {
  return getToken(CelParser::LPAREN, 0);
}

tree::TerminalNode* CelParser::NestedContext::RPAREN() {
  return getToken(CelParser::RPAREN, 0);
}

CelParser::ExprContext* CelParser::NestedContext::expr() {
  return getRuleContext<CelParser::ExprContext>(0);
}

CelParser::NestedContext::NestedContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::NestedContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitNested(this);
  else
    return visitor->visitChildren(this);
}
//----------------- CreateMessageContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::CreateMessageContext::RBRACE() {
  return getToken(CelParser::RBRACE, 0);
}

std::vector<tree::TerminalNode *> CelParser::CreateMessageContext::IDENTIFIER() {
  return getTokens(CelParser::IDENTIFIER);
}

tree::TerminalNode* CelParser::CreateMessageContext::IDENTIFIER(size_t i) {
  return getToken(CelParser::IDENTIFIER, i);
}

tree::TerminalNode* CelParser::CreateMessageContext::LBRACE() {
  return getToken(CelParser::LBRACE, 0);
}

tree::TerminalNode* CelParser::CreateMessageContext::COMMA() {
  return getToken(CelParser::COMMA, 0);
}

std::vector<tree::TerminalNode *> CelParser::CreateMessageContext::DOT() {
  return getTokens(CelParser::DOT);
}

tree::TerminalNode* CelParser::CreateMessageContext::DOT(size_t i) {
  return getToken(CelParser::DOT, i);
}

CelParser::FieldInitializerListContext* CelParser::CreateMessageContext::fieldInitializerList() {
  return getRuleContext<CelParser::FieldInitializerListContext>(0);
}

CelParser::CreateMessageContext::CreateMessageContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::CreateMessageContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitCreateMessage(this);
  else
    return visitor->visitChildren(this);
}
//----------------- GlobalCallContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::GlobalCallContext::IDENTIFIER() {
  return getToken(CelParser::IDENTIFIER, 0);
}

tree::TerminalNode* CelParser::GlobalCallContext::RPAREN() {
  return getToken(CelParser::RPAREN, 0);
}

tree::TerminalNode* CelParser::GlobalCallContext::LPAREN() {
  return getToken(CelParser::LPAREN, 0);
}

tree::TerminalNode* CelParser::GlobalCallContext::DOT() {
  return getToken(CelParser::DOT, 0);
}

CelParser::ExprListContext* CelParser::GlobalCallContext::exprList() {
  return getRuleContext<CelParser::ExprListContext>(0);
}

CelParser::GlobalCallContext::GlobalCallContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::GlobalCallContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitGlobalCall(this);
  else
    return visitor->visitChildren(this);
}
//----------------- CreateMapContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::CreateMapContext::RBRACE() {
  return getToken(CelParser::RBRACE, 0);
}

tree::TerminalNode* CelParser::CreateMapContext::LBRACE() {
  return getToken(CelParser::LBRACE, 0);
}

tree::TerminalNode* CelParser::CreateMapContext::COMMA() {
  return getToken(CelParser::COMMA, 0);
}

CelParser::MapInitializerListContext* CelParser::CreateMapContext::mapInitializerList() {
  return getRuleContext<CelParser::MapInitializerListContext>(0);
}

CelParser::CreateMapContext::CreateMapContext(PrimaryContext *ctx) { copyFrom(ctx); }


std::any CelParser::CreateMapContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitCreateMap(this);
  else
    return visitor->visitChildren(this);
}
CelParser::PrimaryContext* CelParser::primary() {
  PrimaryContext *_localctx = _tracker.createInstance<PrimaryContext>(_ctx, getState());
  enterRule(_localctx, 16, CelParser::RulePrimary);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    setState(184);
    _errHandler->sync(this);
    switch (getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 25, _ctx)) {
    case 1: {
      _localctx = _tracker.createInstance<CelParser::IdentContext>(_localctx);
      enterOuterAlt(_localctx, 1);
      setState(132);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::DOT) {
        setState(131);
        antlrcpp::downCast<IdentContext *>(_localctx)->leadingDot = match(CelParser::DOT);
      }
      setState(134);
      antlrcpp::downCast<IdentContext *>(_localctx)->id = match(CelParser::IDENTIFIER);
      break;
    }

    case 2: {
      _localctx = _tracker.createInstance<CelParser::GlobalCallContext>(_localctx);
      enterOuterAlt(_localctx, 2);
      setState(136);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::DOT) {
        setState(135);
        antlrcpp::downCast<GlobalCallContext *>(_localctx)->leadingDot = match(CelParser::DOT);
      }
      setState(138);
      antlrcpp::downCast<GlobalCallContext *>(_localctx)->id = match(CelParser::IDENTIFIER);

      setState(139);
      antlrcpp::downCast<GlobalCallContext *>(_localctx)->op = match(CelParser::LPAREN);
      setState(141);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if ((((_la & ~ 0x3fULL) == 0) &&
        ((1ULL << _la) & 135762105344) != 0)) {
        setState(140);
        antlrcpp::downCast<GlobalCallContext *>(_localctx)->args = exprList();
      }
      setState(143);
      match(CelParser::RPAREN);
      break;
    }

    case 3: {
      _localctx = _tracker.createInstance<CelParser::NestedContext>(_localctx);
      enterOuterAlt(_localctx, 3);
      setState(144);
      match(CelParser::LPAREN);
      setState(145);
      antlrcpp::downCast<NestedContext *>(_localctx)->e = expr();
      setState(146);
      match(CelParser::RPAREN);
      break;
    }

    case 4: {
      _localctx = _tracker.createInstance<CelParser::CreateListContext>(_localctx);
      enterOuterAlt(_localctx, 4);
      setState(148);
      antlrcpp::downCast<CreateListContext *>(_localctx)->op = match(CelParser::LBRACKET);
      setState(150);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if ((((_la & ~ 0x3fULL) == 0) &&
        ((1ULL << _la) & 135763153920) != 0)) {
        setState(149);
        antlrcpp::downCast<CreateListContext *>(_localctx)->elems = listInit();
      }
      setState(153);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::COMMA) {
        setState(152);
        match(CelParser::COMMA);
      }
      setState(155);
      match(CelParser::RPRACKET);
      break;
    }

    case 5: {
      _localctx = _tracker.createInstance<CelParser::CreateMapContext>(_localctx);
      enterOuterAlt(_localctx, 5);
      setState(156);
      antlrcpp::downCast<CreateMapContext *>(_localctx)->op = match(CelParser::LBRACE);
      setState(158);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if ((((_la & ~ 0x3fULL) == 0) &&
        ((1ULL << _la) & 135763153920) != 0)) {
        setState(157);
        antlrcpp::downCast<CreateMapContext *>(_localctx)->entries = mapInitializerList();
      }
      setState(161);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::COMMA) {
        setState(160);
        match(CelParser::COMMA);
      }
      setState(163);
      match(CelParser::RBRACE);
      break;
    }

    case 6: {
      _localctx = _tracker.createInstance<CelParser::CreateMessageContext>(_localctx);
      enterOuterAlt(_localctx, 6);
      setState(165);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::DOT) {
        setState(164);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->leadingDot = match(CelParser::DOT);
      }
      setState(167);
      antlrcpp::downCast<CreateMessageContext *>(_localctx)->identifierToken = match(CelParser::IDENTIFIER);
      antlrcpp::downCast<CreateMessageContext *>(_localctx)->ids.push_back(antlrcpp::downCast<CreateMessageContext *>(_localctx)->identifierToken);
      setState(172);
      _errHandler->sync(this);
      _la = _input->LA(1);
      while (_la == CelParser::DOT) {
        setState(168);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->s16 = match(CelParser::DOT);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->ops.push_back(antlrcpp::downCast<CreateMessageContext *>(_localctx)->s16);
        setState(169);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->identifierToken = match(CelParser::IDENTIFIER);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->ids.push_back(antlrcpp::downCast<CreateMessageContext *>(_localctx)->identifierToken);
        setState(174);
        _errHandler->sync(this);
        _la = _input->LA(1);
      }
      setState(175);
      antlrcpp::downCast<CreateMessageContext *>(_localctx)->op = match(CelParser::LBRACE);
      setState(177);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if ((((_la & ~ 0x3fULL) == 0) &&
        ((1ULL << _la) & 206159478784) != 0)) {
        setState(176);
        antlrcpp::downCast<CreateMessageContext *>(_localctx)->entries = fieldInitializerList();
      }
      setState(180);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::COMMA) {
        setState(179);
        match(CelParser::COMMA);
      }
      setState(182);
      match(CelParser::RBRACE);
      break;
    }

    case 7: {
      _localctx = _tracker.createInstance<CelParser::ConstantLiteralContext>(_localctx);
      enterOuterAlt(_localctx, 7);
      setState(183);
      literal();
      break;
    }

    default:
      break;
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- ExprListContext ------------------------------------------------------------------

CelParser::ExprListContext::ExprListContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::ExprContext *> CelParser::ExprListContext::expr() {
  return getRuleContexts<CelParser::ExprContext>();
}

CelParser::ExprContext* CelParser::ExprListContext::expr(size_t i) {
  return getRuleContext<CelParser::ExprContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::ExprListContext::COMMA() {
  return getTokens(CelParser::COMMA);
}

tree::TerminalNode* CelParser::ExprListContext::COMMA(size_t i) {
  return getToken(CelParser::COMMA, i);
}


size_t CelParser::ExprListContext::getRuleIndex() const {
  return CelParser::RuleExprList;
}


std::any CelParser::ExprListContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitExprList(this);
  else
    return visitor->visitChildren(this);
}

CelParser::ExprListContext* CelParser::exprList() {
  ExprListContext *_localctx = _tracker.createInstance<ExprListContext>(_ctx, getState());
  enterRule(_localctx, 18, CelParser::RuleExprList);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(186);
    antlrcpp::downCast<ExprListContext *>(_localctx)->exprContext = expr();
    antlrcpp::downCast<ExprListContext *>(_localctx)->e.push_back(antlrcpp::downCast<ExprListContext *>(_localctx)->exprContext);
    setState(191);
    _errHandler->sync(this);
    _la = _input->LA(1);
    while (_la == CelParser::COMMA) {
      setState(187);
      match(CelParser::COMMA);
      setState(188);
      antlrcpp::downCast<ExprListContext *>(_localctx)->exprContext = expr();
      antlrcpp::downCast<ExprListContext *>(_localctx)->e.push_back(antlrcpp::downCast<ExprListContext *>(_localctx)->exprContext);
      setState(193);
      _errHandler->sync(this);
      _la = _input->LA(1);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- ListInitContext ------------------------------------------------------------------

CelParser::ListInitContext::ListInitContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::OptExprContext *> CelParser::ListInitContext::optExpr() {
  return getRuleContexts<CelParser::OptExprContext>();
}

CelParser::OptExprContext* CelParser::ListInitContext::optExpr(size_t i) {
  return getRuleContext<CelParser::OptExprContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::ListInitContext::COMMA() {
  return getTokens(CelParser::COMMA);
}

tree::TerminalNode* CelParser::ListInitContext::COMMA(size_t i) {
  return getToken(CelParser::COMMA, i);
}


size_t CelParser::ListInitContext::getRuleIndex() const {
  return CelParser::RuleListInit;
}


std::any CelParser::ListInitContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitListInit(this);
  else
    return visitor->visitChildren(this);
}

CelParser::ListInitContext* CelParser::listInit() {
  ListInitContext *_localctx = _tracker.createInstance<ListInitContext>(_ctx, getState());
  enterRule(_localctx, 20, CelParser::RuleListInit);

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    setState(194);
    antlrcpp::downCast<ListInitContext *>(_localctx)->optExprContext = optExpr();
    antlrcpp::downCast<ListInitContext *>(_localctx)->elems.push_back(antlrcpp::downCast<ListInitContext *>(_localctx)->optExprContext);
    setState(199);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 27, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        setState(195);
        match(CelParser::COMMA);
        setState(196);
        antlrcpp::downCast<ListInitContext *>(_localctx)->optExprContext = optExpr();
        antlrcpp::downCast<ListInitContext *>(_localctx)->elems.push_back(antlrcpp::downCast<ListInitContext *>(_localctx)->optExprContext); 
      }
      setState(201);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 27, _ctx);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- FieldInitializerListContext ------------------------------------------------------------------

CelParser::FieldInitializerListContext::FieldInitializerListContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::OptFieldContext *> CelParser::FieldInitializerListContext::optField() {
  return getRuleContexts<CelParser::OptFieldContext>();
}

CelParser::OptFieldContext* CelParser::FieldInitializerListContext::optField(size_t i) {
  return getRuleContext<CelParser::OptFieldContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::FieldInitializerListContext::COLON() {
  return getTokens(CelParser::COLON);
}

tree::TerminalNode* CelParser::FieldInitializerListContext::COLON(size_t i) {
  return getToken(CelParser::COLON, i);
}

std::vector<CelParser::ExprContext *> CelParser::FieldInitializerListContext::expr() {
  return getRuleContexts<CelParser::ExprContext>();
}

CelParser::ExprContext* CelParser::FieldInitializerListContext::expr(size_t i) {
  return getRuleContext<CelParser::ExprContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::FieldInitializerListContext::COMMA() {
  return getTokens(CelParser::COMMA);
}

tree::TerminalNode* CelParser::FieldInitializerListContext::COMMA(size_t i) {
  return getToken(CelParser::COMMA, i);
}


size_t CelParser::FieldInitializerListContext::getRuleIndex() const {
  return CelParser::RuleFieldInitializerList;
}


std::any CelParser::FieldInitializerListContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitFieldInitializerList(this);
  else
    return visitor->visitChildren(this);
}

CelParser::FieldInitializerListContext* CelParser::fieldInitializerList() {
  FieldInitializerListContext *_localctx = _tracker.createInstance<FieldInitializerListContext>(_ctx, getState());
  enterRule(_localctx, 22, CelParser::RuleFieldInitializerList);

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    setState(202);
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->optFieldContext = optField();
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->fields.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->optFieldContext);
    setState(203);
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->s21 = match(CelParser::COLON);
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->cols.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->s21);
    setState(204);
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->exprContext = expr();
    antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->values.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->exprContext);
    setState(212);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 28, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        setState(205);
        match(CelParser::COMMA);
        setState(206);
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->optFieldContext = optField();
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->fields.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->optFieldContext);
        setState(207);
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->s21 = match(CelParser::COLON);
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->cols.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->s21);
        setState(208);
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->exprContext = expr();
        antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->values.push_back(antlrcpp::downCast<FieldInitializerListContext *>(_localctx)->exprContext); 
      }
      setState(214);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 28, _ctx);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- OptFieldContext ------------------------------------------------------------------

CelParser::OptFieldContext::OptFieldContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

CelParser::EscapeIdentContext* CelParser::OptFieldContext::escapeIdent() {
  return getRuleContext<CelParser::EscapeIdentContext>(0);
}

tree::TerminalNode* CelParser::OptFieldContext::QUESTIONMARK() {
  return getToken(CelParser::QUESTIONMARK, 0);
}


size_t CelParser::OptFieldContext::getRuleIndex() const {
  return CelParser::RuleOptField;
}


std::any CelParser::OptFieldContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitOptField(this);
  else
    return visitor->visitChildren(this);
}

CelParser::OptFieldContext* CelParser::optField() {
  OptFieldContext *_localctx = _tracker.createInstance<OptFieldContext>(_ctx, getState());
  enterRule(_localctx, 24, CelParser::RuleOptField);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(216);
    _errHandler->sync(this);

    _la = _input->LA(1);
    if (_la == CelParser::QUESTIONMARK) {
      setState(215);
      antlrcpp::downCast<OptFieldContext *>(_localctx)->opt = match(CelParser::QUESTIONMARK);
    }
    setState(218);
    escapeIdent();
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- MapInitializerListContext ------------------------------------------------------------------

CelParser::MapInitializerListContext::MapInitializerListContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

std::vector<CelParser::OptExprContext *> CelParser::MapInitializerListContext::optExpr() {
  return getRuleContexts<CelParser::OptExprContext>();
}

CelParser::OptExprContext* CelParser::MapInitializerListContext::optExpr(size_t i) {
  return getRuleContext<CelParser::OptExprContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::MapInitializerListContext::COLON() {
  return getTokens(CelParser::COLON);
}

tree::TerminalNode* CelParser::MapInitializerListContext::COLON(size_t i) {
  return getToken(CelParser::COLON, i);
}

std::vector<CelParser::ExprContext *> CelParser::MapInitializerListContext::expr() {
  return getRuleContexts<CelParser::ExprContext>();
}

CelParser::ExprContext* CelParser::MapInitializerListContext::expr(size_t i) {
  return getRuleContext<CelParser::ExprContext>(i);
}

std::vector<tree::TerminalNode *> CelParser::MapInitializerListContext::COMMA() {
  return getTokens(CelParser::COMMA);
}

tree::TerminalNode* CelParser::MapInitializerListContext::COMMA(size_t i) {
  return getToken(CelParser::COMMA, i);
}


size_t CelParser::MapInitializerListContext::getRuleIndex() const {
  return CelParser::RuleMapInitializerList;
}


std::any CelParser::MapInitializerListContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitMapInitializerList(this);
  else
    return visitor->visitChildren(this);
}

CelParser::MapInitializerListContext* CelParser::mapInitializerList() {
  MapInitializerListContext *_localctx = _tracker.createInstance<MapInitializerListContext>(_ctx, getState());
  enterRule(_localctx, 26, CelParser::RuleMapInitializerList);

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    size_t alt;
    enterOuterAlt(_localctx, 1);
    setState(220);
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->optExprContext = optExpr();
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->keys.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->optExprContext);
    setState(221);
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->s21 = match(CelParser::COLON);
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->cols.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->s21);
    setState(222);
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->exprContext = expr();
    antlrcpp::downCast<MapInitializerListContext *>(_localctx)->values.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->exprContext);
    setState(230);
    _errHandler->sync(this);
    alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 30, _ctx);
    while (alt != 2 && alt != atn::ATN::INVALID_ALT_NUMBER) {
      if (alt == 1) {
        setState(223);
        match(CelParser::COMMA);
        setState(224);
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->optExprContext = optExpr();
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->keys.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->optExprContext);
        setState(225);
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->s21 = match(CelParser::COLON);
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->cols.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->s21);
        setState(226);
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->exprContext = expr();
        antlrcpp::downCast<MapInitializerListContext *>(_localctx)->values.push_back(antlrcpp::downCast<MapInitializerListContext *>(_localctx)->exprContext); 
      }
      setState(232);
      _errHandler->sync(this);
      alt = getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 30, _ctx);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- EscapeIdentContext ------------------------------------------------------------------

CelParser::EscapeIdentContext::EscapeIdentContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}


size_t CelParser::EscapeIdentContext::getRuleIndex() const {
  return CelParser::RuleEscapeIdent;
}

void CelParser::EscapeIdentContext::copyFrom(EscapeIdentContext *ctx) {
  ParserRuleContext::copyFrom(ctx);
}

//----------------- EscapedIdentifierContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::EscapedIdentifierContext::ESC_IDENTIFIER() {
  return getToken(CelParser::ESC_IDENTIFIER, 0);
}

CelParser::EscapedIdentifierContext::EscapedIdentifierContext(EscapeIdentContext *ctx) { copyFrom(ctx); }


std::any CelParser::EscapedIdentifierContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitEscapedIdentifier(this);
  else
    return visitor->visitChildren(this);
}
//----------------- SimpleIdentifierContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::SimpleIdentifierContext::IDENTIFIER() {
  return getToken(CelParser::IDENTIFIER, 0);
}

CelParser::SimpleIdentifierContext::SimpleIdentifierContext(EscapeIdentContext *ctx) { copyFrom(ctx); }


std::any CelParser::SimpleIdentifierContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitSimpleIdentifier(this);
  else
    return visitor->visitChildren(this);
}
CelParser::EscapeIdentContext* CelParser::escapeIdent() {
  EscapeIdentContext *_localctx = _tracker.createInstance<EscapeIdentContext>(_ctx, getState());
  enterRule(_localctx, 28, CelParser::RuleEscapeIdent);

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    setState(235);
    _errHandler->sync(this);
    switch (_input->LA(1)) {
      case CelParser::IDENTIFIER: {
        _localctx = _tracker.createInstance<CelParser::SimpleIdentifierContext>(_localctx);
        enterOuterAlt(_localctx, 1);
        setState(233);
        antlrcpp::downCast<SimpleIdentifierContext *>(_localctx)->id = match(CelParser::IDENTIFIER);
        break;
      }

      case CelParser::ESC_IDENTIFIER: {
        _localctx = _tracker.createInstance<CelParser::EscapedIdentifierContext>(_localctx);
        enterOuterAlt(_localctx, 2);
        setState(234);
        antlrcpp::downCast<EscapedIdentifierContext *>(_localctx)->id = match(CelParser::ESC_IDENTIFIER);
        break;
      }

    default:
      throw NoViableAltException(this);
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- OptExprContext ------------------------------------------------------------------

CelParser::OptExprContext::OptExprContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}

CelParser::ExprContext* CelParser::OptExprContext::expr() {
  return getRuleContext<CelParser::ExprContext>(0);
}

tree::TerminalNode* CelParser::OptExprContext::QUESTIONMARK() {
  return getToken(CelParser::QUESTIONMARK, 0);
}


size_t CelParser::OptExprContext::getRuleIndex() const {
  return CelParser::RuleOptExpr;
}


std::any CelParser::OptExprContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitOptExpr(this);
  else
    return visitor->visitChildren(this);
}

CelParser::OptExprContext* CelParser::optExpr() {
  OptExprContext *_localctx = _tracker.createInstance<OptExprContext>(_ctx, getState());
  enterRule(_localctx, 30, CelParser::RuleOptExpr);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    enterOuterAlt(_localctx, 1);
    setState(238);
    _errHandler->sync(this);

    _la = _input->LA(1);
    if (_la == CelParser::QUESTIONMARK) {
      setState(237);
      antlrcpp::downCast<OptExprContext *>(_localctx)->opt = match(CelParser::QUESTIONMARK);
    }
    setState(240);
    antlrcpp::downCast<OptExprContext *>(_localctx)->e = expr();
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

//----------------- LiteralContext ------------------------------------------------------------------

CelParser::LiteralContext::LiteralContext(ParserRuleContext *parent, size_t invokingState)
  : ParserRuleContext(parent, invokingState) {
}


size_t CelParser::LiteralContext::getRuleIndex() const {
  return CelParser::RuleLiteral;
}

void CelParser::LiteralContext::copyFrom(LiteralContext *ctx) {
  ParserRuleContext::copyFrom(ctx);
}

//----------------- BytesContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::BytesContext::BYTES() {
  return getToken(CelParser::BYTES, 0);
}

CelParser::BytesContext::BytesContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::BytesContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitBytes(this);
  else
    return visitor->visitChildren(this);
}
//----------------- UintContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::UintContext::NUM_UINT() {
  return getToken(CelParser::NUM_UINT, 0);
}

CelParser::UintContext::UintContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::UintContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitUint(this);
  else
    return visitor->visitChildren(this);
}
//----------------- NullContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::NullContext::NUL() {
  return getToken(CelParser::NUL, 0);
}

CelParser::NullContext::NullContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::NullContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitNull(this);
  else
    return visitor->visitChildren(this);
}
//----------------- BoolFalseContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::BoolFalseContext::CEL_FALSE() {
  return getToken(CelParser::CEL_FALSE, 0);
}

CelParser::BoolFalseContext::BoolFalseContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::BoolFalseContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitBoolFalse(this);
  else
    return visitor->visitChildren(this);
}
//----------------- StringContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::StringContext::STRING() {
  return getToken(CelParser::STRING, 0);
}

CelParser::StringContext::StringContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::StringContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitString(this);
  else
    return visitor->visitChildren(this);
}
//----------------- DoubleContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::DoubleContext::NUM_FLOAT() {
  return getToken(CelParser::NUM_FLOAT, 0);
}

tree::TerminalNode* CelParser::DoubleContext::MINUS() {
  return getToken(CelParser::MINUS, 0);
}

CelParser::DoubleContext::DoubleContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::DoubleContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitDouble(this);
  else
    return visitor->visitChildren(this);
}
//----------------- BoolTrueContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::BoolTrueContext::CEL_TRUE() {
  return getToken(CelParser::CEL_TRUE, 0);
}

CelParser::BoolTrueContext::BoolTrueContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::BoolTrueContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitBoolTrue(this);
  else
    return visitor->visitChildren(this);
}
//----------------- IntContext ------------------------------------------------------------------

tree::TerminalNode* CelParser::IntContext::NUM_INT() {
  return getToken(CelParser::NUM_INT, 0);
}

tree::TerminalNode* CelParser::IntContext::MINUS() {
  return getToken(CelParser::MINUS, 0);
}

CelParser::IntContext::IntContext(LiteralContext *ctx) { copyFrom(ctx); }


std::any CelParser::IntContext::accept(tree::ParseTreeVisitor *visitor) {
  if (auto parserVisitor = dynamic_cast<CelVisitor*>(visitor))
    return parserVisitor->visitInt(this);
  else
    return visitor->visitChildren(this);
}
CelParser::LiteralContext* CelParser::literal() {
  LiteralContext *_localctx = _tracker.createInstance<LiteralContext>(_ctx, getState());
  enterRule(_localctx, 32, CelParser::RuleLiteral);
  size_t _la = 0;

#if __cplusplus > 201703L
  auto onExit = finally([=, this] {
#else
  auto onExit = finally([=] {
#endif
    exitRule();
  });
  try {
    setState(256);
    _errHandler->sync(this);
    switch (getInterpreter<atn::ParserATNSimulator>()->adaptivePredict(_input, 35, _ctx)) {
    case 1: {
      _localctx = _tracker.createInstance<CelParser::IntContext>(_localctx);
      enterOuterAlt(_localctx, 1);
      setState(243);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::MINUS) {
        setState(242);
        antlrcpp::downCast<IntContext *>(_localctx)->sign = match(CelParser::MINUS);
      }
      setState(245);
      antlrcpp::downCast<IntContext *>(_localctx)->tok = match(CelParser::NUM_INT);
      break;
    }

    case 2: {
      _localctx = _tracker.createInstance<CelParser::UintContext>(_localctx);
      enterOuterAlt(_localctx, 2);
      setState(246);
      antlrcpp::downCast<UintContext *>(_localctx)->tok = match(CelParser::NUM_UINT);
      break;
    }

    case 3: {
      _localctx = _tracker.createInstance<CelParser::DoubleContext>(_localctx);
      enterOuterAlt(_localctx, 3);
      setState(248);
      _errHandler->sync(this);

      _la = _input->LA(1);
      if (_la == CelParser::MINUS) {
        setState(247);
        antlrcpp::downCast<DoubleContext *>(_localctx)->sign = match(CelParser::MINUS);
      }
      setState(250);
      antlrcpp::downCast<DoubleContext *>(_localctx)->tok = match(CelParser::NUM_FLOAT);
      break;
    }

    case 4: {
      _localctx = _tracker.createInstance<CelParser::StringContext>(_localctx);
      enterOuterAlt(_localctx, 4);
      setState(251);
      antlrcpp::downCast<StringContext *>(_localctx)->tok = match(CelParser::STRING);
      break;
    }

    case 5: {
      _localctx = _tracker.createInstance<CelParser::BytesContext>(_localctx);
      enterOuterAlt(_localctx, 5);
      setState(252);
      antlrcpp::downCast<BytesContext *>(_localctx)->tok = match(CelParser::BYTES);
      break;
    }

    case 6: {
      _localctx = _tracker.createInstance<CelParser::BoolTrueContext>(_localctx);
      enterOuterAlt(_localctx, 6);
      setState(253);
      antlrcpp::downCast<BoolTrueContext *>(_localctx)->tok = match(CelParser::CEL_TRUE);
      break;
    }

    case 7: {
      _localctx = _tracker.createInstance<CelParser::BoolFalseContext>(_localctx);
      enterOuterAlt(_localctx, 7);
      setState(254);
      antlrcpp::downCast<BoolFalseContext *>(_localctx)->tok = match(CelParser::CEL_FALSE);
      break;
    }

    case 8: {
      _localctx = _tracker.createInstance<CelParser::NullContext>(_localctx);
      enterOuterAlt(_localctx, 8);
      setState(255);
      antlrcpp::downCast<NullContext *>(_localctx)->tok = match(CelParser::NUL);
      break;
    }

    default:
      break;
    }
   
  }
  catch (RecognitionException &e) {
    _errHandler->reportError(this, e);
    _localctx->exception = std::current_exception();
    _errHandler->recover(this, _localctx->exception);
  }

  return _localctx;
}

bool CelParser::sempred(RuleContext *context, size_t ruleIndex, size_t predicateIndex) {
  switch (ruleIndex) {
    case 4: return relationSempred(antlrcpp::downCast<RelationContext *>(context), predicateIndex);
    case 5: return calcSempred(antlrcpp::downCast<CalcContext *>(context), predicateIndex);
    case 7: return memberSempred(antlrcpp::downCast<MemberContext *>(context), predicateIndex);

  default:
    break;
  }
  return true;
}

bool CelParser::relationSempred(RelationContext *_localctx, size_t predicateIndex) {
  switch (predicateIndex) {
    case 0: return precpred(_ctx, 1);

  default:
    break;
  }
  return true;
}

bool CelParser::calcSempred(CalcContext *_localctx, size_t predicateIndex) {
  switch (predicateIndex) {
    case 1: return precpred(_ctx, 2);
    case 2: return precpred(_ctx, 1);

  default:
    break;
  }
  return true;
}

bool CelParser::memberSempred(MemberContext *_localctx, size_t predicateIndex) {
  switch (predicateIndex) {
    case 3: return precpred(_ctx, 3);
    case 4: return precpred(_ctx, 2);
    case 5: return precpred(_ctx, 1);

  default:
    break;
  }
  return true;
}

void CelParser::initialize() {
#if ANTLR4_USE_THREAD_LOCAL_CACHE
  celParserInitialize();
#else
  ::antlr4::internal::call_once(celParserOnceFlag, celParserInitialize);
#endif
}
