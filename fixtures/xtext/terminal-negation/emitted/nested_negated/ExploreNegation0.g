lexer grammar ExploreNegation0;
@members {public int errors=0; @Override public void reportError(RecognitionException e){errors++;}}
RULE_VALUE: ~(~('a'));
