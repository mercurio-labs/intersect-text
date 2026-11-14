lexer grammar NegationAudit21;
@members { public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } }
RULE_VALUE: 'a' ~('b') 'c';
RULE_SHORT: 'a';
RULE_REST: .;
