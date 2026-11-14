package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilValidExplicitParentOperandChildLexer extends Lexer {
    public static final int RULE_END=6;
    public static final int RULE_VALUE=4;
    public static final int SUPER_END=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilValidExplicitParentOperandChildLexer() {;} 
    public InternalUntilValidExplicitParentOperandChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilValidExplicitParentOperandChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilValidExplicitParentOperandChild.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilValidExplicitParentOperandChild.g:98:12: ( '@' ( options {greedy=false; } : . )* SUPER_END )
            // InternalUntilValidExplicitParentOperandChild.g:98:14: '@' ( options {greedy=false; } : . )* SUPER_END
            {
            match('@'); 
            // InternalUntilValidExplicitParentOperandChild.g:98:18: ( options {greedy=false; } : . )*
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
            	    // InternalUntilValidExplicitParentOperandChild.g:98:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            mSUPER_END(); 

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
            // InternalUntilValidExplicitParentOperandChild.g:100:19: ( ( 'ab' | 'abc' ) )
            // InternalUntilValidExplicitParentOperandChild.g:100:21: ( 'ab' | 'abc' )
            {
            // InternalUntilValidExplicitParentOperandChild.g:100:21: ( 'ab' | 'abc' )
            int alt2=2;
            int LA2_0 = input.LA(1);

            if ( (LA2_0=='a') ) {
                int LA2_1 = input.LA(2);

                if ( (LA2_1=='b') ) {
                    int LA2_2 = input.LA(3);

                    if ( (LA2_2=='c') ) {
                        alt2=2;
                    }
                    else {
                        alt2=1;}
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
                    // InternalUntilValidExplicitParentOperandChild.g:100:22: 'ab'
                    {
                    match("ab"); 


                    }
                    break;
                case 2 :
                    // InternalUntilValidExplicitParentOperandChild.g:100:27: 'abc'
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

    // $ANTLR start "SUPER_END"
    public final void mSUPER_END() throws RecognitionException {
        try {
            // InternalUntilValidExplicitParentOperandChild.g:102:20: ( ( 'abc' | 'b' ) )
            // InternalUntilValidExplicitParentOperandChild.g:102:22: ( 'abc' | 'b' )
            {
            // InternalUntilValidExplicitParentOperandChild.g:102:22: ( 'abc' | 'b' )
            int alt3=2;
            int LA3_0 = input.LA(1);

            if ( (LA3_0=='a') ) {
                alt3=1;
            }
            else if ( (LA3_0=='b') ) {
                alt3=2;
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 3, 0, input);

                throw nvae;
            }
            switch (alt3) {
                case 1 :
                    // InternalUntilValidExplicitParentOperandChild.g:102:23: 'abc'
                    {
                    match("abc"); 


                    }
                    break;
                case 2 :
                    // InternalUntilValidExplicitParentOperandChild.g:102:29: 'b'
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
    // $ANTLR end "SUPER_END"

    public void mTokens() throws RecognitionException {
        // InternalUntilValidExplicitParentOperandChild.g:1:8: ( RULE_VALUE )
        // InternalUntilValidExplicitParentOperandChild.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}