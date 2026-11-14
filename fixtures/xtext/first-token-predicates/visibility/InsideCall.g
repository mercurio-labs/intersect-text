grammar InsideCall;
@parser::members {public String branch="";}
entry: (one {branch+="g";} | two {branch+="f";}) EOF; one: ('a')=> 'a' 'b'; two: 'a' 'b';
EXTRA_C: 'c';
WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};
