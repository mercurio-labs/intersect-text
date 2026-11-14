package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilValidInheritedOperandChildLexer extends Lexer {
    public static final int RULE_END=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilValidInheritedOperandChildLexer() {;} 
    public InternalUntilValidInheritedOperandChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilValidInheritedOperandChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilValidInheritedOperandChild.g"; }

    // $ANTLR start "RULE_END"
    public final void mRULE_END() throws RecognitionException {
        try {
            // InternalUntilValidInheritedOperandChild.g:98:19: ( ( 'ab' | 'abc' ) )
            // InternalUntilValidInheritedOperandChild.g:98:21: ( 'ab' | 'abc' )
            {
            // InternalUntilValidInheritedOperandChild.g:98:21: ( 'ab' | 'abc' )
            int alt1=2;
            int LA1_0 = input.LA(1);

            if ( (LA1_0=='a') ) {
                int LA1_1 = input.LA(2);

                if ( (LA1_1=='b') ) {
                    int LA1_2 = input.LA(3);

                    if ( (LA1_2=='c') ) {
                        alt1=2;
                    }
                    else {
                        alt1=1;}
                }
                else {
                    NoViableAltException nvae =
                        new NoViableAltException("", 1, 1, input);

                    throw nvae;
                }
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 1, 0, input);

                throw nvae;
            }
            switch (alt1) {
                case 1 :
                    // InternalUntilValidInheritedOperandChild.g:98:22: 'ab'
                    {
                    match("ab"); 


                    }
                    break;
                case 2 :
                    // InternalUntilValidInheritedOperandChild.g:98:27: 'abc'
                    {
                    match("abc"); 


                    }
                    break;

            }


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_END"

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilValidInheritedOperandChild.g:100:12: ( '@' ( options {greedy=false; } : . )* RULE_END )
            // InternalUntilValidInheritedOperandChild.g:100:14: '@' ( options {greedy=false; } : . )* RULE_END
            {
            match('@'); 
            // InternalUntilValidInheritedOperandChild.g:100:18: ( options {greedy=false; } : . )*
            loop2:
            do {
                int alt2=2;
                int LA2_0 = input.LA(1);

                if ( (LA2_0=='a') ) {
                    int LA2_1 = input.LA(2);

                    if ( (LA2_1=='b') ) {
                        alt2=2;
                    }
                    else if ( ((LA2_1>='\u0000' && LA2_1<='a')||(LA2_1>='c' && LA2_1<='\uFFFF')) ) {
                        alt2=1;
                    }


                }
                else if ( ((LA2_0>='\u0000' && LA2_0<='`')||(LA2_0>='b' && LA2_0<='\uFFFF')) ) {
                    alt2=1;
                }


                switch (alt2) {
            	case 1 :
            	    // InternalUntilValidInheritedOperandChild.g:100:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop2;
                }
            } while (true);

            mRULE_END(); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    public void mTokens() throws RecognitionException {
        // InternalUntilValidInheritedOperandChild.g:1:8: ( RULE_VALUE )
        // InternalUntilValidInheritedOperandChild.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}