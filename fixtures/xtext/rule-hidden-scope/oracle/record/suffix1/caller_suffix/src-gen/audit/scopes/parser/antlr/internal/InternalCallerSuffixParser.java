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
import audit.scopes.services.CallerSuffixGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalCallerSuffixParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_WS", "RULE_ID", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'!'", "'?'", "'direct'", "'done'", "'a'", "'b'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=5;
    public static final int RULE_WS=4;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int T__12=12;
    public static final int T__13=13;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalCallerSuffixParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalCallerSuffixParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalCallerSuffixParser.tokenNames; }
    public String getGrammarFileName() { return "InternalCallerSuffix.g"; }



     	private CallerSuffixGrammarAccess grammarAccess;

        public InternalCallerSuffixParser(TokenStream input, CallerSuffixGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected CallerSuffixGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalCallerSuffix.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalCallerSuffix.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalCallerSuffix.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalCallerSuffix.g:71:1: ruleModel returns [EObject current=null] : ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) | (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_1=null;
        Token otherlv_3=null;
        Token otherlv_4=null;
        Token otherlv_6=null;
        AntlrDatatypeRuleToken lv_name_0_0 = null;

        AntlrDatatypeRuleToken lv_name_2_0 = null;

        EObject this_TightEntry_5 = null;



        	enterRule();

        try {
            // InternalCallerSuffix.g:77:2: ( ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) | (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' ) ) )
            // InternalCallerSuffix.g:78:2: ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) | (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' ) )
            {
            // InternalCallerSuffix.g:78:2: ( ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' ) | ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' ) | (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' ) )
            int alt1=3;
            int LA1_0 = input.LA(1);

            if ( (LA1_0==12) ) {
                int LA1_1 = input.LA(2);

                if ( (LA1_1==13) ) {
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
            else if ( (LA1_0==10) ) {
                alt1=3;
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 1, 0, input);

                throw nvae;
            }
            switch (alt1) {
                case 1 :
                    // InternalCallerSuffix.g:79:3: ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' )
                    {
                    // InternalCallerSuffix.g:79:3: ( ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!' )
                    // InternalCallerSuffix.g:80:4: ( (lv_name_0_0= ruleLoose ) ) otherlv_1= '!'
                    {
                    // InternalCallerSuffix.g:80:4: ( (lv_name_0_0= ruleLoose ) )
                    // InternalCallerSuffix.g:81:5: (lv_name_0_0= ruleLoose )
                    {
                    // InternalCallerSuffix.g:81:5: (lv_name_0_0= ruleLoose )
                    // InternalCallerSuffix.g:82:6: lv_name_0_0= ruleLoose
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
                    							"audit.scopes.CallerSuffix.Loose");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }

                    otherlv_1=(Token)match(input,8,FOLLOW_2); 

                    				newLeafNode(otherlv_1, grammarAccess.getModelAccess().getExclamationMarkKeyword_0_1());
                    			

                    }


                    }
                    break;
                case 2 :
                    // InternalCallerSuffix.g:105:3: ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' )
                    {
                    // InternalCallerSuffix.g:105:3: ( ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?' )
                    // InternalCallerSuffix.g:106:4: ( (lv_name_2_0= ruleTight ) ) otherlv_3= '?'
                    {
                    // InternalCallerSuffix.g:106:4: ( (lv_name_2_0= ruleTight ) )
                    // InternalCallerSuffix.g:107:5: (lv_name_2_0= ruleTight )
                    {
                    // InternalCallerSuffix.g:107:5: (lv_name_2_0= ruleTight )
                    // InternalCallerSuffix.g:108:6: lv_name_2_0= ruleTight
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
                    							"audit.scopes.CallerSuffix.Tight");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }

                    otherlv_3=(Token)match(input,9,FOLLOW_2); 

                    				newLeafNode(otherlv_3, grammarAccess.getModelAccess().getQuestionMarkKeyword_1_1());
                    			

                    }


                    }
                    break;
                case 3 :
                    // InternalCallerSuffix.g:131:3: (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' )
                    {
                    // InternalCallerSuffix.g:131:3: (otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done' )
                    // InternalCallerSuffix.g:132:4: otherlv_4= 'direct' this_TightEntry_5= ruleTightEntry otherlv_6= 'done'
                    {
                    otherlv_4=(Token)match(input,10,FOLLOW_5); 

                    				newLeafNode(otherlv_4, grammarAccess.getModelAccess().getDirectKeyword_2_0());
                    			

                    				newCompositeNode(grammarAccess.getModelAccess().getTightEntryParserRuleCall_2_1());
                    			
                    pushFollow(FOLLOW_6);
                    this_TightEntry_5=ruleTightEntry();

                    state._fsp--;


                    				current = this_TightEntry_5;
                    				afterParserOrEnumRuleCall();
                    			
                    otherlv_6=(Token)match(input,11,FOLLOW_2); 

                    				newLeafNode(otherlv_6, grammarAccess.getModelAccess().getDoneKeyword_2_2());
                    			

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


    // $ANTLR start "entryRuleTightEntry"
    // InternalCallerSuffix.g:153:1: entryRuleTightEntry returns [EObject current=null] : iv_ruleTightEntry= ruleTightEntry EOF ;
    public final EObject entryRuleTightEntry() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleTightEntry = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalCallerSuffix.g:155:2: (iv_ruleTightEntry= ruleTightEntry EOF )
            // InternalCallerSuffix.g:156:2: iv_ruleTightEntry= ruleTightEntry EOF
            {
             newCompositeNode(grammarAccess.getTightEntryRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleTightEntry=ruleTightEntry();

            state._fsp--;

             current =iv_ruleTightEntry; 
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
    // $ANTLR end "entryRuleTightEntry"


    // $ANTLR start "ruleTightEntry"
    // InternalCallerSuffix.g:165:1: ruleTightEntry returns [EObject current=null] : ( ( (lv_name_0_0= ruleTight ) ) otherlv_1= '?' ) ;
    public final EObject ruleTightEntry() throws RecognitionException {
        EObject current = null;

        Token otherlv_1=null;
        AntlrDatatypeRuleToken lv_name_0_0 = null;



        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalCallerSuffix.g:172:2: ( ( ( (lv_name_0_0= ruleTight ) ) otherlv_1= '?' ) )
            // InternalCallerSuffix.g:173:2: ( ( (lv_name_0_0= ruleTight ) ) otherlv_1= '?' )
            {
            // InternalCallerSuffix.g:173:2: ( ( (lv_name_0_0= ruleTight ) ) otherlv_1= '?' )
            // InternalCallerSuffix.g:174:3: ( (lv_name_0_0= ruleTight ) ) otherlv_1= '?'
            {
            // InternalCallerSuffix.g:174:3: ( (lv_name_0_0= ruleTight ) )
            // InternalCallerSuffix.g:175:4: (lv_name_0_0= ruleTight )
            {
            // InternalCallerSuffix.g:175:4: (lv_name_0_0= ruleTight )
            // InternalCallerSuffix.g:176:5: lv_name_0_0= ruleTight
            {

            					newCompositeNode(grammarAccess.getTightEntryAccess().getNameTightParserRuleCall_0_0());
            				
            pushFollow(FOLLOW_4);
            lv_name_0_0=ruleTight();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getTightEntryRule());
            					}
            					set(
            						current,
            						"name",
            						lv_name_0_0,
            						"audit.scopes.CallerSuffix.Tight");
            					afterParserOrEnumRuleCall();
            				

            }


            }

            otherlv_1=(Token)match(input,9,FOLLOW_2); 

            			newLeafNode(otherlv_1, grammarAccess.getTightEntryAccess().getQuestionMarkKeyword_1());
            		

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
    // $ANTLR end "ruleTightEntry"


    // $ANTLR start "entryRuleLoose"
    // InternalCallerSuffix.g:204:1: entryRuleLoose returns [String current=null] : iv_ruleLoose= ruleLoose EOF ;
    public final String entryRuleLoose() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleLoose = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalCallerSuffix.g:206:2: (iv_ruleLoose= ruleLoose EOF )
            // InternalCallerSuffix.g:207:2: iv_ruleLoose= ruleLoose EOF
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
    // InternalCallerSuffix.g:216:1: ruleLoose returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' kw= 'b' ) ;
    public final AntlrDatatypeRuleToken ruleLoose() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalCallerSuffix.g:223:2: ( (kw= 'a' kw= 'b' ) )
            // InternalCallerSuffix.g:224:2: (kw= 'a' kw= 'b' )
            {
            // InternalCallerSuffix.g:224:2: (kw= 'a' kw= 'b' )
            // InternalCallerSuffix.g:225:3: kw= 'a' kw= 'b'
            {
            kw=(Token)match(input,12,FOLLOW_7); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getLooseAccess().getAKeyword_0());
            		
            kw=(Token)match(input,13,FOLLOW_2); 

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
    // InternalCallerSuffix.g:242:1: entryRuleTight returns [String current=null] : iv_ruleTight= ruleTight EOF ;
    public final String entryRuleTight() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleTight = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalCallerSuffix.g:244:2: (iv_ruleTight= ruleTight EOF )
            // InternalCallerSuffix.g:245:2: iv_ruleTight= ruleTight EOF
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
    // InternalCallerSuffix.g:254:1: ruleTight returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' this_WS_1= RULE_WS kw= 'b' ) ;
    public final AntlrDatatypeRuleToken ruleTight() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;
        Token this_WS_1=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalCallerSuffix.g:261:2: ( (kw= 'a' this_WS_1= RULE_WS kw= 'b' ) )
            // InternalCallerSuffix.g:262:2: (kw= 'a' this_WS_1= RULE_WS kw= 'b' )
            {
            // InternalCallerSuffix.g:262:2: (kw= 'a' this_WS_1= RULE_WS kw= 'b' )
            // InternalCallerSuffix.g:263:3: kw= 'a' this_WS_1= RULE_WS kw= 'b'
            {
            kw=(Token)match(input,12,FOLLOW_8); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getTightAccess().getAKeyword_0());
            		
            this_WS_1=(Token)match(input,RULE_WS,FOLLOW_7); 

            			current.merge(this_WS_1);
            		

            			newLeafNode(this_WS_1, grammarAccess.getTightAccess().getWSTerminalRuleCall_1());
            		
            kw=(Token)match(input,13,FOLLOW_2); 

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
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000001000L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000000800L});
    public static final BitSet FOLLOW_7 = new BitSet(new long[]{0x0000000000002000L});
    public static final BitSet FOLLOW_8 = new BitSet(new long[]{0x0000000000000010L});

}