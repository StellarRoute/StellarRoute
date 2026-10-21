/**
 * Card Component Fixtures
 * 
 * Test data and example props for card components.
 * Use these in tests, storybooks, or development.
 */

export const cardFaceFixtures = {
  active: {
    last4: '4567',
    expiry: '12/25',
    usdcAvailable: '1,234.56',
    isFrozen: false,
  },
  frozen: {
    last4: '4567',
    expiry: '12/25',
    usdcAvailable: '1,234.56',
    isFrozen: true,
  },
  lowBalance: {
    last4: '8901',
    expiry: '03/26',
    usdcAvailable: '10.50',
    isFrozen: false,
  },
  highBalance: {
    last4: '2345',
    expiry: '09/27',
    usdcAvailable: '50,000.00',
    isFrozen: false,
  },
  zeroBalance: {
    last4: '6789',
    expiry: '06/25',
    usdcAvailable: '0.00',
    isFrozen: false,
  },
  expiringSoon: {
    last4: '1111',
    expiry: '01/27',
    usdcAvailable: '500.00',
    isFrozen: false,
  },
};

export const cardConfirmFixtures = {
  smallPurchase: {
    merchant: 'Coffee Shop',
    fiatAmount: '5.50',
    fiatCurrency: 'USD',
    usdcGross: '5.48',
    fee: '0.02',
    exchangeRate: '1.004',
  },
  mediumPurchase: {
    merchant: 'Electronics Store',
    fiatAmount: '299.99',
    fiatCurrency: 'USD',
    usdcGross: '298.50',
    fee: '1.49',
    exchangeRate: '1.005',
  },
  largePurchase: {
    merchant: 'Furniture Retailer',
    fiatAmount: '2,500.00',
    fiatCurrency: 'USD',
    usdcGross: '2,487.50',
    fee: '12.50',
    exchangeRate: '1.005',
  },
  euroTransaction: {
    merchant: 'Paris Boutique',
    fiatAmount: '150.00',
    fiatCurrency: 'EUR',
    usdcGross: '165.00',
    fee: '0.83',
    exchangeRate: '0.909',
  },
  zeroFee: {
    merchant: 'Promotional Partner',
    fiatAmount: '100.00',
    fiatCurrency: 'USD',
    usdcGross: '100.00',
    fee: '0.00',
    exchangeRate: '1.000',
  },
  highFee: {
    merchant: 'International Vendor',
    fiatAmount: '1,000.00',
    fiatCurrency: 'USD',
    usdcGross: '985.00',
    fee: '15.00',
    exchangeRate: '1.015',
  },
};

export const viewFullDetailsFixtures = {
  defaultPartner: {
    issuerPartnerUrl: 'https://partner.example.com/card-details',
  },
  namedPartner: {
    issuerPartnerName: 'Acme Bank',
    issuerPartnerUrl: 'https://acmebank.example.com/cards/virtual',
  },
  testPartner: {
    issuerPartnerName: 'Test Issuer',
    issuerPartnerUrl: 'https://test-issuer.dev/cards',
  },
};

/**
 * Complete card scenario combining multiple components
 */
export const cardScenarios = {
  newUser: {
    card: cardFaceFixtures.lowBalance,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  activeUser: {
    card: cardFaceFixtures.active,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  frozenAccount: {
    card: cardFaceFixtures.frozen,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  highValueUser: {
    card: cardFaceFixtures.highBalance,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
};

/**
 * Transaction flow scenarios with both card state and confirmation
 */
export const transactionScenarios = {
  coffeePurchase: {
    card: cardFaceFixtures.active,
    confirmation: cardConfirmFixtures.smallPurchase,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  electronicsPurchase: {
    card: cardFaceFixtures.highBalance,
    confirmation: cardConfirmFixtures.mediumPurchase,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  furniturePurchase: {
    card: cardFaceFixtures.highBalance,
    confirmation: cardConfirmFixtures.largePurchase,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  internationalPurchase: {
    card: cardFaceFixtures.active,
    confirmation: cardConfirmFixtures.euroTransaction,
    issuer: viewFullDetailsFixtures.namedPartner,
  },
  insufficientBalance: {
    card: cardFaceFixtures.lowBalance,
    confirmation: cardConfirmFixtures.mediumPurchase,
    issuer: viewFullDetailsFixtures.namedPartner,
    error: 'Insufficient USDC balance',
  },
  frozenCardAttempt: {
    card: cardFaceFixtures.frozen,
    confirmation: cardConfirmFixtures.smallPurchase,
    issuer: viewFullDetailsFixtures.namedPartner,
    error: 'Card is frozen - spending paused',
  },
};

export default {
  cardFace: cardFaceFixtures,
  cardConfirm: cardConfirmFixtures,
  viewFullDetails: viewFullDetailsFixtures,
  scenarios: cardScenarios,
  transactions: transactionScenarios,
};
