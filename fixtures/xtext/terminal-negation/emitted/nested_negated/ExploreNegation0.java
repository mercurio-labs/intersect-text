// $ANTLR 3.2 Sep 23, 2009 12:02:23 D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g 2026-10-17 21:47:01

import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

public class ExploreNegation0 extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;
    public int errors=0; @Override public void reportError(RecognitionException e){errors++;}

    // delegates
    // delegators

    public ExploreNegation0() {;} 
    public ExploreNegation0(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public ExploreNegation0(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g:3:11: (~ (~ ( 'a' ) ) )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g:3:13: ~ (~ ( 'a' ) )
            {
            if ( input.LA(1)=='a' ) {
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

    public void mTokens() throws RecognitionException {
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g:1:8: ( RULE_VALUE )
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\nested_negated\\ExploreNegation0.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}