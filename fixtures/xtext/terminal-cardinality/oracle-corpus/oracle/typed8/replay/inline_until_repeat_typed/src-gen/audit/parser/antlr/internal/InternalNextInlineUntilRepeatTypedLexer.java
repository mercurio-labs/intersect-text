package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalNextInlineUntilRepeatTypedLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalNextInlineUntilRepeatTypedLexer() {;} 
    public InternalNextInlineUntilRepeatTypedLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalNextInlineUntilRepeatTypedLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalNextInlineUntilRepeatTyped.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalNextInlineUntilRepeatTyped.g:98:12: ( ( '@' ( options {greedy=false; } : . )* 'ab' )+ 'c' )
            // InternalNextInlineUntilRepeatTyped.g:98:14: ( '@' ( options {greedy=false; } : . )* 'ab' )+ 'c'
            {
            // InternalNextInlineUntilRepeatTyped.g:98:14: ( '@' ( options {greedy=false; } : . )* 'ab' )+
            int cnt2=0;
            loop2:
            do {
                int alt2=2;
                int LA2_0 = input.LA(1);

                if ( (LA2_0=='@') ) {
                    alt2=1;
                }


                switch (alt2) {
            	case 1 :
            	    // InternalNextInlineUntilRepeatTyped.g:98:15: '@' ( options {greedy=false; } : . )* 'ab'
            	    {
            	    match('@'); 
            	    // InternalNextInlineUntilRepeatTyped.g:98:19: ( options {greedy=false; } : . )*
            	    loop1:
            	    do {
            	        int alt1=2;
            	        int LA1_0 = input.LA(1);

            	        if ( (LA1_0=='a') ) {
            	            int LA1_1 = input.LA(2);

            	            if ( (LA1_1=='b') ) {
            	                int LA1_3 = input.LA(3);

            	                if ( (LA1_3=='c') ) {
            	                    alt1=2;
            	                }
            	                else if ( (LA1_3=='@') ) {
            	                    alt1=2;
            	                }
            	                else if ( ((LA1_3>='\u0000' && LA1_3<='?')||(LA1_3>='A' && LA1_3<='b')||(LA1_3>='d' && LA1_3<='\uFFFF')) ) {
            	                    alt1=1;
            	                }


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
            	    	    // InternalNextInlineUntilRepeatTyped.g:98:47: .
            	    	    {
            	    	    matchAny(); 

            	    	    }
            	    	    break;

            	    	default :
            	    	    break loop1;
            	        }
            	    } while (true);

            	    match("ab"); 


            	    }
            	    break;

            	default :
            	    if ( cnt2 >= 1 ) break loop2;
                        EarlyExitException eee =
                            new EarlyExitException(2, input);
                        throw eee;
                }
                cnt2++;
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

    public void mTokens() throws RecognitionException {
        // InternalNextInlineUntilRepeatTyped.g:1:8: ( RULE_VALUE )
        // InternalNextInlineUntilRepeatTyped.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}