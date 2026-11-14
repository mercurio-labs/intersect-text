# Complete KerML draft fixture

The three definitions are byte-identical files from the upstream SysML v2 Pilot
Implementation revision recorded in sources.json. Original copyright and license
headers are retained; UPSTREAM-LICENSE preserves the repository license.

controls.rs passes the complete original Ecore and both complete Xtext grammars
through native migration and ordinary preparation. The empty authored behavior
context installs no domain rules. Seven operand declarations are admission
contracts only; their implementations are not installed in the draft consumer.
No grammar projection, Java, checkout path, I/O or network is used at runtime.

Success qualifies only this source-bound physical draft construction. All class
and member semantic obligations and all 20/80 basic workflows remain unverified.
