grammar AfterAction;
@parser::members {public String branch="";}
entry: ({branch+="p";} ('a')=> 'a' 'b' {branch+="g";} | 'a' 'b' {branch+="f";}) EOF;
EXTRA_C: 'c';
WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};
