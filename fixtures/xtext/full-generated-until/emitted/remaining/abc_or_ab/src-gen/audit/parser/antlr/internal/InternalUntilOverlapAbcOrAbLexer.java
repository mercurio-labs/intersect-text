package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilOverlapAbcOrAbLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilOverlapAbcOrAbLexer() {;} 
    public InternalUntilOverlapAbcOrAbLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilOverlapAbcOrAbLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilOverlapAbcOrAb.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilOverlapAbcOrAb.g:98:12: ( '@' ( options {greedy=false; } : . )* ( 'abc' | 'ab' ) )
            // InternalUntilOverlapAbcOrAb.g:98:14: '@' ( options {greedy=false; } : . )* ( 'abc' | 'ab' )
            {
            match('@'); 
            // InternalUntilOverlapAbcOrAb.g:98:18: ( options {greedy=false; } : . )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='a') ) {
                    int LA1_1 = input.LA(2);

                    if ( (LA1_1=='b') ) {
                        alt1=2;
                    }
                    else if ( ((LA1_1>='\u0000' && LA1_1<='a')||(LA1_1>='c' && LA1_1<='\uFFFF')) ) {
                        alt1=1;
                    }


                }
                else if ( ((LA1_0>='\u0000' && LA1_0<='`')||(LA1_0>='b' && LA1_0<='\uFFFF')) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalUntilOverlapAbcOrAb.g:98:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            // InternalUntilOverlapAbcOrAb.g:98:50: ( 'abc' | 'ab' )
            int alt2=2;
            int LA2_0 = input.LA(1);

            if ( (LA2_0=='a') ) {
                int LA2_1 = input.LA(2);

                if ( (LA2_1=='b') ) {
                    int LA2_2 = input.LA(3);

                    if ( (LA2_2=='c') ) {
                        alt2=1;
                    }
                    else {
                        alt2=2;}
                }
                else {
                    NoViableAltException nvae =
                        new NoViableAltException("", 2, 1, input);

                    throw nvae;
                }
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 2, 0, input);

                throw nvae;
            }
            switch (alt2) {
                case 1 :
                    // InternalUntilOverlapAbcOrAb.g:98:51: 'abc'
                    {
                    match("abc"); 


                    }
                    break;
                case 2 :
                    // InternalUntilOverlapAbcOrAb.g:98:57: 'ab'
                    {
                    match("ab"); 


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
        // InternalUntilOverlapAbcOrAb.g:1:8: ( RULE_VALUE )
        // InternalUntilOverlapAbcOrAb.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}