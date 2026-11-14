package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFiniteUntilAlternativesCompanionLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalFiniteUntilAlternativesCompanionLexer() {;} 
    public InternalFiniteUntilAlternativesCompanionLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalFiniteUntilAlternativesCompanionLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalFiniteUntilAlternativesCompanion.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalFiniteUntilAlternativesCompanion.g:98:12: ( '@' ( options {greedy=false; } : . )* ( 'END' | 'STOP' ) )
            // InternalFiniteUntilAlternativesCompanion.g:98:14: '@' ( options {greedy=false; } : . )* ( 'END' | 'STOP' )
            {
            match('@'); 
            // InternalFiniteUntilAlternativesCompanion.g:98:18: ( options {greedy=false; } : . )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='E') ) {
                    int LA1_1 = input.LA(2);

                    if ( (LA1_1=='N') ) {
                        int LA1_4 = input.LA(3);

                        if ( (LA1_4=='D') ) {
                            alt1=2;
                        }
                        else if ( ((LA1_4>='\u0000' && LA1_4<='C')||(LA1_4>='E' && LA1_4<='\uFFFF')) ) {
                            alt1=1;
                        }


                    }
                    else if ( ((LA1_1>='\u0000' && LA1_1<='M')||(LA1_1>='O' && LA1_1<='\uFFFF')) ) {
                        alt1=1;
                    }


                }
                else if ( (LA1_0=='S') ) {
                    int LA1_2 = input.LA(2);

                    if ( (LA1_2=='T') ) {
                        int LA1_5 = input.LA(3);

                        if ( (LA1_5=='O') ) {
                            int LA1_7 = input.LA(4);

                            if ( (LA1_7=='P') ) {
                                alt1=2;
                            }
                            else if ( ((LA1_7>='\u0000' && LA1_7<='O')||(LA1_7>='Q' && LA1_7<='\uFFFF')) ) {
                                alt1=1;
                            }


                        }
                        else if ( ((LA1_5>='\u0000' && LA1_5<='N')||(LA1_5>='P' && LA1_5<='\uFFFF')) ) {
                            alt1=1;
                        }


                    }
                    else if ( ((LA1_2>='\u0000' && LA1_2<='S')||(LA1_2>='U' && LA1_2<='\uFFFF')) ) {
                        alt1=1;
                    }


                }
                else if ( ((LA1_0>='\u0000' && LA1_0<='D')||(LA1_0>='F' && LA1_0<='R')||(LA1_0>='T' && LA1_0<='\uFFFF')) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalFiniteUntilAlternativesCompanion.g:98:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            // InternalFiniteUntilAlternativesCompanion.g:98:50: ( 'END' | 'STOP' )
            int alt2=2;
            int LA2_0 = input.LA(1);

            if ( (LA2_0=='E') ) {
                alt2=1;
            }
            else if ( (LA2_0=='S') ) {
                alt2=2;
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 2, 0, input);

                throw nvae;
            }
            switch (alt2) {
                case 1 :
                    // InternalFiniteUntilAlternativesCompanion.g:98:51: 'END'
                    {
                    match("END"); 


                    }
                    break;
                case 2 :
                    // InternalFiniteUntilAlternativesCompanion.g:98:57: 'STOP'
                    {
                    match("STOP"); 


                    }
                    break;

            }


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    public void mTokens() throws RecognitionException {
        // InternalFiniteUntilAlternativesCompanion.g:1:8: ( RULE_VALUE )
        // InternalFiniteUntilAlternativesCompanion.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}