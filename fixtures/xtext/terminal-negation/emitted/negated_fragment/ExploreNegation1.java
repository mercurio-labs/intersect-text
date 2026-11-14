// $ANTLR 3.2 Sep 23, 2009 12:02:23 D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g 2026-10-17 21:47:04

import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

public class ExploreNegation1 extends Lexer {
    public static final int RULE_LETTER=4;
    public static final int RULE_VALUE=5;
    public static final int EOF=-1;
    public int errors=0; @Override public void reportError(RecognitionException e){errors++;}

    // delegates
    // delegators

    public ExploreNegation1() {;} 
    public ExploreNegation1(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public ExploreNegation1(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:3:11: (~ ( RULE_LETTER ) )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:3:13: ~ ( RULE_LETTER )
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

    // $ANTLR start "RULE_LETTER"
    public final void mRULE_LETTER() throws RecognitionException {
        try {
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:4:21: (~ ( 'a' ) )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:4:23: ~ ( 'a' )
            {
            if ( (input.LA(1)>='\u0000' && input.LA(1)<='`')||(input.LA(1)>='b' && input.LA(1)<='\uFFFF') ) {
                input.consume();

            }
            else {
                MismatchedSetException mse = new MismatchedSetException(null,input);
                recover(mse);
                throw mse;}


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_LETTER"

    public void mTokens() throws RecognitionException {
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:1:8: ( RULE_VALUE )
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-exploratory-nested-v1\\negated_fragment\\ExploreNegation1.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}