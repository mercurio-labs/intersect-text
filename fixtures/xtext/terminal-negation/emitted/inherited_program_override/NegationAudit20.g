lexer grammar NegationAudit20;
@members { public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } }
fragment RULE_LETTER: 'b';
RULE_VALUE: ~(RULE_LETTER);
