package audit.scopes.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.scopes.services.PredictionMarkerScopeGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalPredictionMarkerScopeParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_WS", "RULE_ID", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'!'", "'?'", "'a'", "'b'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=5;
    public static final int RULE_WS=4;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalPredictionMarkerScopeParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalPredictionMarkerScopeParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalPredictionMarkerScopeParser.tokenNames; }
    public String getGrammarFileName() { return "InternalPredictionMarkerScope.g"; }



     	private PredictionMarkerScopeGrammarAccess grammarAccess;

        public InternalPredictionMarkerScopeParser(TokenStream input, PredictionMarkerScopeGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected PredictionMarkerScopeGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalPredictionMarkerScope.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalPredictionMarkerScope.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalPredictionMarkerScope.g:65:2: iv_ruleModel= ruleModel EOF
            {
             newCompositeNode(grammarAccess.getModelRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleModel=ruleModel();

            state._fsp--;

             current =iv_ruleModel; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleModel"


    // $ANTLR start "ruleModel"
    // InternalPredictionMarkerScope.g:71:1: ruleModel returns [EObject current=null] : ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_1=null;
        Token otherlv_3=null;
        AntlrDatatypeRuleToken lv_name_0_0 = null;

        AntlrDatatypeRuleToken lv_name_2_0 = null;



        	enterRule();

        try {
            // InternalPredictionMarkerScope.g:77:2: ( ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) ) )
            // InternalPredictionMarkerScope.g:78:2: ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) )
            {
            // InternalPredictionMarkerScope.g:78:2: ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) )
            int alt1=2;
            int LA1_0 = input.LA(1);

            if ( (LA1_0==10) ) {
                int LA1_1 = input.LA(2);

                if ( (LA1_1==11) ) {
                    alt1=1;
                }
                else if ( (LA1_1==RULE_WS) ) {
                    alt1=2;
                }
                else {
                    NoViableAltException nvae =
                        new NoViableAltException("", 1, 1, input);

                    throw nvae;
                }
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 1, 0, input);

                throw nvae;
            }
            switch (alt1) {
                case 1 :
                    // InternalPredictionMarkerScope.g:79:3: ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' )
                    {
                    // InternalPredictionMarkerScope.g:79:3: ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' )
                    // InternalPredictionMarkerScope.g:80:4: ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!'
                    {
                    // InternalPredictionMarkerScope.g:80:4: ( (lv_name_0_0= ruleLoose ) )
                    // InternalPredictionMarkerScope.g:81:5: (lv_name_0_0= ruleLoose )
                    {
                    // InternalPredictionMarkerScope.g:81:5: (lv_name_0_0= ruleLoose )
                    // InternalPredictionMarkerScope.g:82:6: lv_name_0_0= ruleLoose
                    {

                    						newCompositeNode(grammarAccess.getModelAccess().getNameLooseParserRuleCall_0_0_0());
                    					
                    pushFollow(FOLLOW_3);
                    lv_name_0_0=ruleLoose();

                    state._fsp--;


                    						if (current==null) {
                    							current = createModelElementForParent(grammarAccess.getModelRule());
                    						}
                    						set(
                    							current,
                    							"name",
                    							lv_name_0_0,
                    							"audit.scopes.PredictionMarkerScope.Loose");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }

                    otherlv_1=(Token)match(input,8,FOLLOW_2); 

                    				newLeafNode(otherlv_1, grammarAccess.getModelAccess().getExclamationMarkKeyword_0_1());
                    			

                    }


                    }
                    break;
                case 2 :
                    // InternalPredictionMarkerScope.g:105:3: ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' )
                    {
                    // InternalPredictionMarkerScope.g:105:3: ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' )
                    // InternalPredictionMarkerScope.g:106:4: ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?'
                    {
                    // InternalPredictionMarkerScope.g:106:4: ( (lv_name_2_0= ruleTight ) )
                    // InternalPredictionMarkerScope.g:107:5: (lv_name_2_0= ruleTight )
                    {
                    // InternalPredictionMarkerScope.g:107:5: (lv_name_2_0= ruleTight )
                    // InternalPredictionMarkerScope.g:108:6: lv_name_2_0= ruleTight
                    {

                    						newCompositeNode(grammarAccess.getModelAccess().getNameTightParserRuleCall_1_0_0());
                    					
                    pushFollow(FOLLOW_4);
                    lv_name_2_0=ruleTight();

                    state._fsp--;


                    						if (current==null) {
                    							current = createModelElementForParent(grammarAccess.getModelRule());
                    						}
                    						set(
                    							current,
                    							"name",
                    							lv_name_2_0,
                    							"audit.scopes.PredictionMarkerScope.Tight");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }

                    otherlv_3=(Token)match(input,9,FOLLOW_2); 

                    				newLeafNode(otherlv_3, grammarAccess.getModelAccess().getQuestionMarkKeyword_1_1());
                    			

                    }


                    }
                    break;

            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleModel"


    // $ANTLR start "entryRuleLoose"
    // InternalPredictionMarkerScope.g:134:1: entryRuleLoose returns [String current=null] : iv_ruleLoose= ruleLoose EOF ;
    public final String entryRuleLoose() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleLoose = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalPredictionMarkerScope.g:136:2: (iv_ruleLoose= ruleLoose EOF )
            // InternalPredictionMarkerScope.g:137:2: iv_ruleLoose= ruleLoose EOF
            {
             newCompositeNode(grammarAccess.getLooseRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleLoose=ruleLoose();

            state._fsp--;

             current =iv_ruleLoose.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "entryRuleLoose"


    // $ANTLR start "ruleLoose"
    // InternalPredictionMarkerScope.g:146:1: ruleLoose returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' kw= 'b' ) ;
    public final AntlrDatatypeRuleToken ruleLoose() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalPredictionMarkerScope.g:153:2: ( (kw= 'a' kw= 'b' ) )
            // InternalPredictionMarkerScope.g:154:2: (kw= 'a' kw= 'b' )
            {
            // InternalPredictionMarkerScope.g:154:2: (kw= 'a' kw= 'b' )
            // InternalPredictionMarkerScope.g:155:3: kw= 'a' kw= 'b'
            {
            kw=(Token)match(input,10,FOLLOW_5); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getLooseAccess().getAKeyword_0());
            		
            kw=(Token)match(input,11,FOLLOW_2); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getLooseAccess().getBKeyword_1());
            		

            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleLoose"


    // $ANTLR start "entryRuleTight"
    // InternalPredictionMarkerScope.g:172:1: entryRuleTight returns [String current=null] : iv_ruleTight= ruleTight EOF ;
    public final String entryRuleTight() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleTight = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalPredictionMarkerScope.g:174:2: (iv_ruleTight= ruleTight EOF )
            // InternalPredictionMarkerScope.g:175:2: iv_ruleTight= ruleTight EOF
            {
             newCompositeNode(grammarAccess.getTightRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleTight=ruleTight();

            state._fsp--;

             current =iv_ruleTight.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "entryRuleTight"


    // $ANTLR start "ruleTight"
    // InternalPredictionMarkerScope.g:184:1: ruleTight returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' this_WS_1= RULE_WS kw= 'b' ) ;
    public final AntlrDatatypeRuleToken ruleTight() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;
        Token this_WS_1=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalPredictionMarkerScope.g:191:2: ( (kw= 'a' this_WS_1= RULE_WS kw= 'b' ) )
            // InternalPredictionMarkerScope.g:192:2: (kw= 'a' this_WS_1= RULE_WS kw= 'b' )
            {
            // InternalPredictionMarkerScope.g:192:2: (kw= 'a' this_WS_1= RULE_WS kw= 'b' )
            // InternalPredictionMarkerScope.g:193:3: kw= 'a' this_WS_1= RULE_WS kw= 'b'
            {
            kw=(Token)match(input,10,FOLLOW_6); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getTightAccess().getAKeyword_0());
            		
            this_WS_1=(Token)match(input,RULE_WS,FOLLOW_5); 

            			current.merge(this_WS_1);
            		

            			newLeafNode(this_WS_1, grammarAccess.getTightAccess().getWSTerminalRuleCall_1());
            		
            kw=(Token)match(input,11,FOLLOW_2); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getTightAccess().getBKeyword_2());
            		

            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleTight"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000100L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000200L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000800L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000000010L});

}