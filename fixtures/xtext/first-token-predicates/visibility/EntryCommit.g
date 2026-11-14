grammar EntryCommit;
@parser::members {public String branch="";}
entry: (('a')=> {branch+="g";} 'a' 'b' | {branch+="f";} 'a' 'b') EOF;
EXTRA_C:'c';
WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};
