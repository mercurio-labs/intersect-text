lexer grammar ExploreNegation1;
@members {public int errors=0; @Override public void reportError(RecognitionException e){errors++;}}
RULE_VALUE: ~(RULE_LETTER);
fragment RULE_LETTER: ~('a');
