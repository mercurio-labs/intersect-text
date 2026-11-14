package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalGroupedMultiNegationLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_PAIR=5;
    public static final int EOF=-1;
    public static final int RULE_INNER=6;

    // delegates
    // delegators

    public InternalGroupedMultiNegationLexer() {;} 
    public InternalGroupedMultiNegationLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalGroupedMultiNegationLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalGroupedMultiNegation.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalGroupedMultiNegation.g:98:12: (~ ( RULE_PAIR ) )
            // InternalGroupedMultiNegation.g:98:14: ~ ( RULE_PAIR )
            {
            if ( (input.LA(1)>='\u0000' && input.LA(1)<='\uFFFF') ) {
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
            // InternalGroupedMultiNegation.g:100:20: ( RULE_INNER )
            // InternalGroupedMultiNegation.g:100:22: RULE_INNER
            {
            mRULE_INNER(); 

            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_PAIR"

    // $ANTLR start "RULE_INNER"
    public final void mRULE_INNER() throws RecognitionException {
        try {
            // InternalGroupedMultiNegation.g:102:21: ( 'ab' )
            // InternalGroupedMultiNegation.g:102:23: 'ab'
            {
            match("ab"); 


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_INNER"

    public void mTokens() throws RecognitionException {
        // InternalGroupedMultiNegation.g:1:8: ( RULE_VALUE )
        // InternalGroupedMultiNegation.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}