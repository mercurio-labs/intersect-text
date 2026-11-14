package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilValidInheritedOperandBaseLexer extends Lexer {
    public static final int RULE_END=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilValidInheritedOperandBaseLexer() {;} 
    public InternalUntilValidInheritedOperandBaseLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilValidInheritedOperandBaseLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilValidInheritedOperandBase.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilValidInheritedOperandBase.g:98:12: ( '@' ( options {greedy=false; } : . )* RULE_END )
            // InternalUntilValidInheritedOperandBase.g:98:14: '@' ( options {greedy=false; } : . )* RULE_END
            {
            match('@'); 
            // InternalUntilValidInheritedOperandBase.g:98:18: ( options {greedy=false; } : . )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='a') ) {
                    int LA1_1 = input.LA(2);

                    if ( (LA1_1=='b') ) {
                        alt1=1;
                    }
                    else if ( ((LA1_1>='\u0000' && LA1_1<='a')||(LA1_1>='c' && LA1_1<='\uFFFF')) ) {
                        alt1=1;
                    }


                }
                else if ( (LA1_0=='b') ) {
                    alt1=2;
                }
                else if ( ((LA1_0>='\u0000' && LA1_0<='`')||(LA1_0>='c' && LA1_0<='\uFFFF')) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalUntilValidInheritedOperandBase.g:98:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop1;
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

    // $ANTLR start "RULE_END"
    public final void mRULE_END() throws RecognitionException {
        try {
            // InternalUntilValidInheritedOperandBase.g:100:19: ( ( 'abc' | 'b' ) )
            // InternalUntilValidInheritedOperandBase.g:100:21: ( 'abc' | 'b' )
            {
            // InternalUntilValidInheritedOperandBase.g:100:21: ( 'abc' | 'b' )
            int alt2=2;
            int LA2_0 = input.LA(1);

            if ( (LA2_0=='a') ) {
                alt2=1;
            }
            else if ( (LA2_0=='b') ) {
                alt2=2;
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 2, 0, input);

                throw nvae;
            }
            switch (alt2) {
                case 1 :
                    // InternalUntilValidInheritedOperandBase.g:100:22: 'abc'
                    {
                    match("abc"); 


                    }
                    break;
                case 2 :
                    // InternalUntilValidInheritedOperandBase.g:100:28: 'b'
                    {
                    match('b'); 

                    }
                    break;

            }


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_END"

    public void mTokens() throws RecognitionException {
        // InternalUntilValidInheritedOperandBase.g:1:8: ( RULE_VALUE )
        // InternalUntilValidInheritedOperandBase.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}