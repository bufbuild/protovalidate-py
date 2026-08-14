
// Generated from external/cel-cpp+/parser/internal/Cel.g4 by ANTLR 4.13.2

#pragma once


#include "antlr4-runtime.h"


namespace cel_parser_internal {


class  CelLexer : public antlr4::Lexer {
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

  explicit CelLexer(antlr4::CharStream *input);

  ~CelLexer() override;


  std::string getGrammarFileName() const override;

  const std::vector<std::string>& getRuleNames() const override;

  const std::vector<std::string>& getChannelNames() const override;

  const std::vector<std::string>& getModeNames() const override;

  const antlr4::dfa::Vocabulary& getVocabulary() const override;

  antlr4::atn::SerializedATNView getSerializedATN() const override;

  const antlr4::atn::ATN& getATN() const override;

  // By default the static state used to implement the lexer is lazily initialized during the first
  // call to the constructor. You can call this function if you wish to initialize the static state
  // ahead of time.
  static void initialize();

private:

  // Individual action functions triggered by action() above.

  // Individual semantic predicate functions triggered by sempred() above.

};

}  // namespace cel_parser_internal
