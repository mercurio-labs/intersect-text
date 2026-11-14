package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalNextInlineStarFollowTypedLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalNextInlineStarFollowTypedLexer() {;} 
    public InternalNextInlineStarFollowTypedLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalNextInlineStarFollowTypedLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalNextInlineStarFollowTyped.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalNextInlineStarFollowTyped.g:98:12: ( ( 'a' )* 'ab' )
            // InternalNextInlineStarFollowTyped.g:98:14: ( 'a' )* 'ab'
            {
            // InternalNextInlineStarFollowTyped.g:98:14: ( 'a' )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='a') ) {
                    int LA1_1 = input.LA(2);

                    if ( (LA1_1=='a') ) {
                        alt1=1;
                    }


                }


                switch (alt1) {
            	case 1 :
            	    // InternalNextInlineStarFollowTyped.g:98:14: 'a'
            	    {
            	    match('a'); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            match("ab"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    public void mTokens() throws RecognitionException {
        // InternalNextInlineStarFollowTyped.g:1:8: ( RULE_VALUE )
        // InternalNextInlineStarFollowTyped.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}