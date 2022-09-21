const nextBuildId = require('next-build-id');

module.exports = {
  generateBuildId: () => nextBuildId({ dir: __dirname, describe: true }),
  webpack: (config, {
    buildId, webpack,
  }) => {
    config.plugins.push(new webpack.DefinePlugin({ buildId: JSON.stringify(buildId) }));
    return config;
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
};
