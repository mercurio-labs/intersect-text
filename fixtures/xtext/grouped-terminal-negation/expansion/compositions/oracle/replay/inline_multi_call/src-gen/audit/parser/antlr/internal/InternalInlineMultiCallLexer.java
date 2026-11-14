package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalInlineMultiCallLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_PAIR=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalInlineMultiCallLexer() {;} 
    public InternalInlineMultiCallLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalInlineMultiCallLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalInlineMultiCall.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalInlineMultiCall.g:98:12: (~ ( ( RULE_PAIR | 'x' ) ) )
            // InternalInlineMultiCall.g:98:14: ~ ( ( RULE_PAIR | 'x' ) )
            {
            if ( (input.LA(1)>='\u0000' && input.LA(1)<='w')||(input.LA(1)>='y' && input.LA(1)<='\uFFFF') ) {
                input.consume();

            }
            else {
                MismatchedSetException mse = new MismatchedSetException(null,input);
                recover(mse);
                throw mse;}


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_PAIR"
    public final void mRULE_PAIR() throws RecognitionException {
        try {
            // InternalInlineMultiCall.g:100:20: ( 'ab' )
            // InternalInlineMultiCall.g:100:22: 'ab'
            {
            match("ab"); 


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_PAIR"

    public void mTokens() throws RecognitionException {
        // InternalInlineMultiCall.g:1:8: ( RULE_VALUE )
        // InternalInlineMultiCall.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}