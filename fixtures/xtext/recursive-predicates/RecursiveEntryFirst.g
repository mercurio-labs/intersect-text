grammar RecursiveEntryFirst;
@parser::members {public String branch="";}
entry: (('a')=> {branch+="g";} 'a' inner 'b' | {branch+="f";} 'a' inner 'c') EOF;
inner: ('a' inner 'b')?;
WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};
