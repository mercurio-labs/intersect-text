package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalNextFragmentUntilRepeatFragmentTypedLexer extends Lexer {
    public static final int RULE_BLOCK=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalNextFragmentUntilRepeatFragmentTypedLexer() {;} 
    public InternalNextFragmentUntilRepeatFragmentTypedLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalNextFragmentUntilRepeatFragmentTypedLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalNextFragmentUntilRepeatFragmentTyped.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalNextFragmentUntilRepeatFragmentTyped.g:98:12: ( ( RULE_BLOCK )+ 'c' )
            // InternalNextFragmentUntilRepeatFragmentTyped.g:98:14: ( RULE_BLOCK )+ 'c'
            {
            // InternalNextFragmentUntilRepeatFragmentTyped.g:98:14: ( RULE_BLOCK )+
            int cnt1=0;
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='@') ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalNextFragmentUntilRepeatFragmentTyped.g:98:14: RULE_BLOCK
            	    {
            	    mRULE_BLOCK(); 

            	    }
            	    break;

            	default :
            	    if ( cnt1 >= 1 ) break loop1;
                        EarlyExitException eee =
                            new EarlyExitException(1, input);
                        throw eee;
                }
                cnt1++;
            } while (true);

            match('c'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_BLOCK"
    public final void mRULE_BLOCK() throws RecognitionException {
        try {
            // InternalNextFragmentUntilRepeatFragmentTyped.g:100:21: ( '@' ( options {greedy=false; } : . )* 'ab' )
            // InternalNextFragmentUntilRepeatFragmentTyped.g:100:23: '@' ( options {greedy=false; } : . )* 'ab'
            {
            match('@'); 
            // InternalNextFragmentUntilRepeatFragmentTyped.g:100:27: ( options {greedy=false; } : . )*
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
            	    // InternalNextFragmentUntilRepeatFragmentTyped.g:100:55: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop2;
                }
            } while (true);

            match("ab"); 


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_BLOCK"

    public void mTokens() throws RecognitionException {
        // InternalNextFragmentUntilRepeatFragmentTyped.g:1:8: ( RULE_VALUE )
        // InternalNextFragmentUntilRepeatFragmentTyped.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}