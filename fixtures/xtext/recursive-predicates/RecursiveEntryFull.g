grammar RecursiveEntryFull;
@parser::members {public String branch="";}
entry: (('a' inner 'b')=> {branch+="g";} 'a' inner 'b' | {branch+="f";} 'a' inner 'c') EOF;
inner: ('a' inner 'b')?;
WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};
