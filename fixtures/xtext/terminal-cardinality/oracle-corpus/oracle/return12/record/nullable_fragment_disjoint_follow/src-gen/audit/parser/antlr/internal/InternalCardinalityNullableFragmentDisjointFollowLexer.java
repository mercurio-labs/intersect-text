package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalCardinalityNullableFragmentDisjointFollowLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_BODY=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalCardinalityNullableFragmentDisjointFollowLexer() {;} 
    public InternalCardinalityNullableFragmentDisjointFollowLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalCardinalityNullableFragmentDisjointFollowLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalCardinalityNullableFragmentDisjointFollow.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCardinalityNullableFragmentDisjointFollow.g:98:12: ( RULE_BODY 'b' )
            // InternalCardinalityNullableFragmentDisjointFollow.g:98:14: RULE_BODY 'b'
            {
            mRULE_BODY(); 
            match('b'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_BODY"
    public final void mRULE_BODY() throws RecognitionException {
        try {
            // InternalCardinalityNullableFragmentDisjointFollow.g:100:20: ( ( 'a' )* )
            // InternalCardinalityNullableFragmentDisjointFollow.g:100:22: ( 'a' )*
            {
            // InternalCardinalityNullableFragmentDisjointFollow.g:100:22: ( 'a' )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='a') ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalCardinalityNullableFragmentDisjointFollow.g:100:22: 'a'
            	    {
            	    match('a'); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_BODY"

    public void mTokens() throws RecognitionException {
        // InternalCardinalityNullableFragmentDisjointFollow.g:1:8: ( RULE_VALUE )
        // InternalCardinalityNullableFragmentDisjointFollow.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}