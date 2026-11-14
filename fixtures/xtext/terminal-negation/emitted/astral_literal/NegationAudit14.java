// $ANTLR 3.2 Sep 23, 2009 12:02:23 D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g 2026-10-17 21:40:26

import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

public class NegationAudit14 extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;
     public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } 

    // delegates
    // delegators

    public NegationAudit14() {;} 
    public NegationAudit14(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public NegationAudit14(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g:3:11: (~ ( '\\uD83C\\uDF0D' ) )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g:3:13: ~ ( '\\uD83C\\uDF0D' )
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

    public void mTokens() throws RecognitionException {
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g:1:8: ( RULE_VALUE )
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\astral_literal\\NegationAudit14.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}