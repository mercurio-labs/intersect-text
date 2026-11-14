package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalExplicitMultiChildLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_PAIR=6;
    public static final int EOF=-1;
    public static final int SUPER_PAIR=5;

    // delegates
    // delegators

    public InternalExplicitMultiChildLexer() {;} 
    public InternalExplicitMultiChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalExplicitMultiChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalExplicitMultiChild.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalExplicitMultiChild.g:98:12: (~ ( SUPER_PAIR ) )
            // InternalExplicitMultiChild.g:98:14: ~ ( SUPER_PAIR )
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
            // InternalExplicitMultiChild.g:100:20: ( 'x' )
            // InternalExplicitMultiChild.g:100:22: 'x'
            {
            match('x'); 

            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_PAIR"

    // $ANTLR start "SUPER_PAIR"
    public final void mSUPER_PAIR() throws RecognitionException {
        try {
            // InternalExplicitMultiChild.g:102:21: ( 'ab' )
            // InternalExplicitMultiChild.g:102:23: 'ab'
            {
            match("ab"); 


            }

        }
        finally {
        }
    }
    // $ANTLR end "SUPER_PAIR"

    public void mTokens() throws RecognitionException {
        // InternalExplicitMultiChild.g:1:8: ( RULE_VALUE )
        // InternalExplicitMultiChild.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}