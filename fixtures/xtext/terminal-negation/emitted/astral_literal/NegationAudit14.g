lexer grammar NegationAudit14;
@members { public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } }
RULE_VALUE: ~('\uD83C\uDF0D');
